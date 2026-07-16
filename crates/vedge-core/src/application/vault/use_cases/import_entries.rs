//! `import_entries` — read a portable file back into the vault (slice 5.3b).
//!
//! The return leg of 5.3a's one-way door. Three sources, one staging model:
//!
//! - **Encrypted** `.vedgex` — `envelope::open` → the plaintext tar → `entries.json`
//!   (the [`ExportBundle`]) + `blobs/{id}` document bytes.
//! - **CSV** — the frozen logins-only adapter; each ok row becomes an
//!   `ExportEntry::Login`, each malformed row a positioned preview error.
//! - **Paste** — the same CSV text, typed into `VEdge` (RAM only, never a file) —
//!   the real no-disk path (Decision ⑧). Reaches [`begin_import`] as `Csv { text }`.
//!
//! # Memory hygiene (research note ⑤)
//!
//! [`ImportSession`] is `#[derive(Zeroize, ZeroizeOnDrop)]` — the **first** in the
//! repo. The staged secrets live as one `Zeroizing<Vec<u8>>` (the serialized
//! bundle) plus per-document `Zeroizing<Vec<u8>>` blobs, capacity-bounded; the
//! derivatives-only preview is `#[zeroize(skip)]` (it holds no secret). Structured
//! secrets are materialized only transiently — at `begin` to derive the preview,
//! and at `commit` to write — so plaintext structs never outlive a call. The
//! session is a field on `VaultSession` (see `session.rs`), so it dies with the
//! lock / TTL and zeroizes as part of that drop.
//!
//! # The preview crosses derivatives only (Decision ⑦)
//!
//! [`ImportPreviewRow`] carries `has_password: bool` (the 4.2 `has_totp` shape) —
//! never the secret. The commit happens entirely here in Rust.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use secrecy::ExposeSecret;
use tracing::instrument;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::session::VaultSession;
use crate::domain::export::dto::{
    EXPORT_FORMAT_VERSION, ExportBundle, ExportEntry, ExportLogin, ExportMeta, ExportTotpParams,
};
use crate::domain::export::mapping::{
    ImportedDocument, ImportedEntry, common_meta_from_export, export_entry_type, export_to_payload,
    payload_to_export,
};
use crate::domain::shared::{EntryId, StorageError, TagId, now};
use crate::domain::vault::aad::{blob_aad, entry_aad, tag_aad};
use crate::domain::vault::crypto_constants::{DEK_LEN, NONCE_LEN};
use crate::domain::vault::entities::{AuditAction, EntryRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, TagPayload};
use crate::infrastructure::export::{archive, csv, envelope};
use crate::infrastructure::snapshot::manifest::{
    SNAPSHOT_VAULT_FILE, SnapshotManifest, verify_hash_prefix,
};
use crate::infrastructure::snapshot::store;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use super::create_entry::{CreateEntryInput, append_audit, create_entry};
use super::document_ops::{ImportDocumentInput, import_document};
use super::tag_ops::create_tag;

/// A picked row: `(preview row_id, bundle entry index)`.
type Pick = (u32, usize);

// ---------------------------------------------------------------------------
// Wire-adjacent, derivatives-only preview (Decision ⑦). The tauri DTO layer
// mirrors these into `vedge-ipc`; NOTHING here carries a secret.
// ---------------------------------------------------------------------------

/// A per-row status badge for the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowStatus {
    /// Nothing to flag.
    Ok,
    /// A non-blocking notice — e.g. a folder absent from the file (imports flat),
    /// or a near-duplicate that shares a base domain with another row.
    Warning(String),
    /// A row that cannot be imported (a malformed CSV row). `line` is the 1-based
    /// source line for a badge.
    Error { message: String, line: Option<u64> },
}

/// One preview row. Derivatives only — `has_password` is the non-invertible
/// presence flag (like 4.2's `has_totp`); the secret never crosses.
#[derive(Debug, Clone)]
pub struct ImportPreviewRow {
    pub row_id: u32,
    pub entry_type: String,
    pub name: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub has_password: bool,
    pub status: RowStatus,
    /// The `row_id` of an earlier row this one likely duplicates (same normalized
    /// host). Advisory — the import **never** auto-merges (research note ②).
    pub duplicate_of: Option<u32>,
}

/// Where the import reads from. The shell reads the file (or takes pasted text)
/// and hands over bytes/text — this use case does no filesystem I/O.
pub enum ImportSource {
    /// A sealed `.vedgex` envelope + the passphrase to open it.
    Encrypted {
        data: Vec<u8>,
        passphrase: Zeroizing<String>,
    },
    /// Plaintext CSV — from a picked file, or pasted into `VEdge` (Decision ⑧).
    Csv { text: String },
}

/// A user's decision for one preview row.
#[derive(Debug, Clone, Copy)]
pub enum RowAction {
    Import,
    Skip,
}

/// One `{row_id, action}` decision from the preview.
#[derive(Debug, Clone, Copy)]
pub struct ImportAction {
    pub row_id: u32,
    pub action: RowAction,
}

/// A row that failed to commit (best-effort import — the batch is not aborted).
#[derive(Debug, Clone)]
pub struct ImportFailure {
    pub row_id: u32,
    pub message: String,
}

/// The outcome of a commit. `imported` + `failed.len()` ≤ picked; `skipped`
/// counts explicit Skips (and uncommittable rows are simply never picked).
#[derive(Debug, Clone)]
pub struct ImportReport {
    pub imported: u64,
    pub skipped: u64,
    pub failed: Vec<ImportFailure>,
}

// ---------------------------------------------------------------------------
// The staging session — the first ZeroizeOnDrop in the repo.
// ---------------------------------------------------------------------------

/// Decrypted-but-not-yet-committed import data, held in RAM while the user
/// reviews the preview. Zeroizes on drop (cancel, lock, or TTL).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ImportSession {
    /// The serialized [`ExportBundle`] of every committable entry. Re-parsed at
    /// commit so structured secrets never outlive a call. The derive zeroizes it.
    bundle_json: Zeroizing<Vec<u8>>,
    /// Document plaintext, keyed by source id (`(id, bytes)`). Each blob is a
    /// self-zeroizing `Zeroizing<Vec<u8>>`, so the derive skips the (non-`Zeroize`)
    /// tuple container and lets the inner buffers wipe on drop.
    #[zeroize(skip)]
    blobs: Vec<(String, Zeroizing<Vec<u8>>)>,
    /// The derivatives-only preview — no secrets, so it is not zeroized.
    #[zeroize(skip)]
    preview: Vec<ImportPreviewRow>,
    /// `preview` `row_id` → index into the bundle's entries (`None` =
    /// uncommittable, e.g. a malformed CSV row).
    #[zeroize(skip)]
    entry_of: Vec<Option<u32>>,
}

impl ImportSession {
    /// The derivatives-only preview rows (what crosses to WASM after DTO mapping).
    #[must_use]
    pub fn preview(&self) -> &[ImportPreviewRow] {
        &self.preview
    }
}

// ---------------------------------------------------------------------------
// begin_import — parse a source into staging + preview. No vault needed.
// ---------------------------------------------------------------------------

/// Begin an import: parse the source, stash the staging on the session, and return
/// the derivatives-only preview.
///
/// The staging lives on the session (so it dies with the lock / TTL). Any prior
/// in-progress import is replaced (the old staging drops → zeroizes).
pub fn begin_import(
    session: &mut VaultSession,
    source: ImportSource,
) -> Result<&[ImportPreviewRow], VaultError> {
    let staging = build_staging(source)?;
    session.import = Some(staging);
    Ok(session.import.as_ref().map_or(&[], ImportSession::preview))
}

/// Discard an in-progress import — the staging drops immediately and zeroizes.
pub fn cancel_import(session: &mut VaultSession) {
    session.import = None;
}

fn build_staging(source: ImportSource) -> Result<ImportSession, VaultError> {
    match source {
        ImportSource::Encrypted { data, passphrase } => {
            let tar = envelope::open(passphrase.as_bytes(), &data)?;
            begin_from_tar(&tar)
        }
        ImportSource::Csv { text } => begin_from_csv(&text),
    }
}

fn begin_from_tar(tar: &[u8]) -> Result<ImportSession, VaultError> {
    let entries_json = archive::read_member(tar, archive::ENTRIES_MEMBER)?
        .ok_or_else(|| VaultError::ExportMalformed("export is missing entries.json".to_owned()))?;

    let bundle: ExportBundle = serde_json::from_slice(&entries_json)
        .map_err(|e| VaultError::ExportMalformed(format!("entries.json: {e}")))?;
    // 🔴 ⑥ the bundle's own version guard is `>`, not `!=` — refuse newer, accept older.
    if bundle.format_version > EXPORT_FORMAT_VERSION {
        return Err(VaultError::ExportUnsupportedFormat(bundle.format_version));
    }

    // Folder source-ids present in THIS file (for the "will import flat" notice).
    let folder_ids: HashSet<String> = bundle
        .entries
        .iter()
        .filter(|e| matches!(e, ExportEntry::Folder(_)))
        .filter_map(|e| e.id().map(str::to_owned))
        .collect();

    let mut blobs: Vec<(String, Zeroizing<Vec<u8>>)> = Vec::new();
    let mut preview = Vec::with_capacity(bundle.entries.len());
    let mut entry_of = Vec::with_capacity(bundle.entries.len());

    for (i, entry) in bundle.entries.iter().enumerate() {
        let row_id = u32::try_from(i).unwrap_or(u32::MAX);

        // A document's plaintext bytes ride as a `blobs/{id}` tar member.
        if let ExportEntry::Document(d) = entry {
            let name = format!("{}{}", archive::BLOBS_PREFIX, d.meta.id);
            if let Some(bytes) = archive::read_member(tar, &name)? {
                blobs.push((d.meta.id.clone(), bytes));
            }
        }

        let mut r = preview_row(row_id, entry);
        if let Some(fid) = source_folder_id(entry) {
            if !folder_ids.contains(fid) {
                r.status = flat_folder_warning();
            }
        }
        preview.push(r);
        entry_of.push(Some(row_id));
    }

    apply_dedup(&mut preview);
    Ok(ImportSession {
        bundle_json: entries_json,
        blobs,
        preview,
        entry_of,
    })
}

fn begin_from_csv(text: &str) -> Result<ImportSession, VaultError> {
    let rows = csv::parse_csv(text.as_bytes());
    let mut entries: Vec<ExportEntry> = Vec::with_capacity(rows.len());
    let mut preview = Vec::with_capacity(rows.len());
    let mut entry_of = Vec::with_capacity(rows.len());

    for (i, csv_row) in rows.iter().enumerate() {
        let row_id = u32::try_from(i).unwrap_or(u32::MAX);
        match &csv_row.result {
            Ok(login) => {
                let entry = csv_login_to_export(row_id, login);
                let bundle_idx = u32::try_from(entries.len()).unwrap_or(u32::MAX);
                preview.push(preview_row(row_id, &entry));
                entries.push(entry);
                entry_of.push(Some(bundle_idx));
            }
            Err(message) => {
                preview.push(ImportPreviewRow {
                    row_id,
                    entry_type: "login".to_owned(),
                    name: String::new(),
                    username: None,
                    url: None,
                    tags: Vec::new(),
                    has_password: false,
                    status: RowStatus::Error {
                        message: message.clone(),
                        line: Some(csv_row.line),
                    },
                    duplicate_of: None,
                });
                entry_of.push(None);
            }
        }
    }

    apply_dedup(&mut preview);
    let bundle_json = Zeroizing::new(
        serde_json::to_vec(&ExportBundle::new(entries))
            .map_err(|e| VaultError::MalformedPayload(format!("csv bundle: {e}")))?,
    );
    Ok(ImportSession {
        bundle_json,
        blobs: Vec::new(),
        preview,
        entry_of,
    })
}

fn csv_login_to_export(row_id: u32, login: &csv::CsvLogin) -> ExportEntry {
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_owned());
    ExportEntry::Login(ExportLogin {
        meta: ExportMeta {
            // A synthetic batch-local id — never persisted (commit mints a fresh ULID).
            id: format!("csv-{row_id}"),
            name: login.name.clone(),
            url: opt(&login.url),
            favicon_url: None,
            tags: login.tags.clone(),
            folder_id: None,
            is_favorite: false,
            notes: opt(&login.notes),
            color: None,
            icon: None,
            sort_order: 0,
        },
        username: login.username.clone(),
        password: login.password.clone(),
        totp_secret: None,
        totp_params: ExportTotpParams::default(),
        recovery_codes: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// begin_snapshot_import — the tweezers (slice 5.3c): a local snapshot as a source.
// ---------------------------------------------------------------------------

/// Begin an import sourced from one of this vault's local snapshots — recover a
/// deleted entry without reverting the whole vault (the tweezers, Decision 🟢).
///
/// Reads the snapshot's `vault.vdb` with the CURRENT session KEK — the rewrap on every
/// credential change (5.2.1 ⑬) keeps every non-stale snapshot's DEKs wrapped under it, so
/// an unlocked session already holds the key. Each entry is mapped into the permanent
/// export DTO and staged through the SAME model [`begin_import`] uses, so `commit_import`
/// re-creates each pick under a **fresh ULID + fresh DEK** in the live vault — nothing is
/// replaced and no re-key is undone; a wrong pick is one entry, not a vault.
///
/// A **stale** snapshot (its `verify_hash_prefix` no longer matches the live vault — a
/// failed rewrap ⑬) is refused with [`VaultError::SnapshotStale`], never opened silently.
///
/// Unlike [`begin_import`] this is async (it opens the snapshot DB), so it is a separate
/// entry point rather than an [`ImportSource`] arm; `commit_import`/`cancel_import` are shared.
#[instrument(skip_all, fields(vault_id = %session.vault_id(), snapshot_id = %snapshot_id))]
pub async fn begin_snapshot_import<'a>(
    session: &'a mut VaultSession,
    snapshot_id: &str,
) -> Result<&'a [ImportPreviewRow], VaultError> {
    let staging = build_snapshot_staging(session, snapshot_id).await?;
    session.import = Some(staging);
    Ok(session.import.as_ref().map_or(&[], ImportSession::preview))
}

async fn build_snapshot_staging(
    session: &VaultSession,
    snapshot_id: &str,
) -> Result<ImportSession, VaultError> {
    // 1. Resolve the (untrusted) id against this vault's store + read its manifest.
    let snapshots_dir = session.vault_id().snapshots_dir();
    let dir = store::resolve_snapshot_dir(&snapshots_dir, snapshot_id)?;
    let manifest = store::read_manifest(&dir)?;

    // 2. Stale guard FIRST — a snapshot whose verify_hash_prefix differs from the live
    //    vault failed a rewrap (⑬); its DEKs are under an older KEK this session cannot
    //    unwrap. Refuse loudly (Decision: never a silent skip).
    if manifest.verify_hash_prefix != verify_hash_prefix(&session.config.verify_hash) {
        return Err(VaultError::SnapshotStale);
    }

    // 3. Read the snapshot DB WITHOUT mutating it: copy vault.vdb into a tempdir and
    //    open+migrate the COPY. vault.vdb is ciphertext (blobs live in the object pool,
    //    not in it), so no plaintext touches disk (②); the tempdir auto-deletes on drop.
    let tmp = tempfile::tempdir()
        .map_err(|e| VaultError::Storage(StorageError::Io(format!("snapshot tempdir: {e}"))))?;
    let tmp_vault = tmp.path().join(SNAPSHOT_VAULT_FILE);
    std::fs::copy(dir.join(SNAPSHOT_VAULT_FILE), &tmp_vault)
        .map_err(|e| VaultError::Storage(StorageError::Io(format!("copy snapshot vault: {e}"))))?;
    let db = VaultDbConnection::open(&tmp_vault).await?;
    let repo = SqliteVaultRepository::new(db.handle());

    let result = read_snapshot_entries(session, &repo, &manifest, &snapshots_dir).await;

    // Release the DB handle BEFORE the tempdir cleanup (Windows won't remove an open file).
    // Best-effort: the read already succeeded, and the copy is a throwaway.
    drop(repo);
    db.close().await.ok();
    let (entries, blobs) = result?;

    stage_from_entries(entries, blobs)
}

/// Decrypt every active entry (+ its document plaintext) of an opened snapshot repo into
/// the export DTO, using the live session's KEK. An ENTRY-DEK decrypt failure that slips past
/// the prefix guard is surfaced as `SnapshotStale` (belt-and-braces — never a silent skip);
/// tag-name resolution is best-effort (see [`read_snapshot_tag_names`]).
async fn read_snapshot_entries(
    session: &VaultSession,
    repo: &SqliteVaultRepository,
    manifest: &SnapshotManifest,
    snapshots_dir: &Path,
) -> Result<(Vec<ExportEntry>, Vec<(String, Zeroizing<Vec<u8>>)>), VaultError> {
    let tag_names_by_id = read_snapshot_tag_names(session, repo).await;

    let rows = repo.all_entries().await?;
    let mut entries: Vec<ExportEntry> = Vec::with_capacity(rows.len());
    let mut blobs: Vec<(String, Zeroizing<Vec<u8>>)> = Vec::new();
    for row in &rows {
        if row.is_trashed {
            continue; // recover LIVE entries only — trash has its own restore (3.6)
        }
        let dek = session
            .crypto
            .unwrap_dek(&row.dek_wrapped, session.kek.expose())
            .map_err(snapshot_kek_error)?;
        let payload = super::refs::decrypt_row_with_dek(session.crypto.as_ref(), &dek, row)
            .map_err(snapshot_kek_error)?;

        if let EntryPayload::Document(doc) = &payload {
            if let Some(bytes) = read_snapshot_blob(
                manifest,
                snapshots_dir,
                &row.id,
                session.crypto.as_ref(),
                &dek,
                &doc.blob_nonce,
            )? {
                blobs.push((row.id.as_str().to_owned(), bytes));
            }
        }

        let tag_names = payload
            .meta()
            .tag_ids
            .iter()
            .filter_map(|tid| tag_names_by_id.get(tid).cloned())
            .collect();
        entries.push(payload_to_export(&row.id, &payload, tag_names));
    }
    Ok((entries, blobs))
}

/// Resolve the snapshot's tag id → name table under the live KEK — **best-effort**.
///
/// Tags are sealed directly under the KEK, and the ⑬ rewrap re-wraps only entry DEKs +
/// `verify_hash`, NOT tag rows (see `rewrap_snapshots`). So a snapshot taken before a credential
/// change carries tag rows under the OLD KEK even once its prefix reads "current". The ENTRIES
/// still recover under the current KEK, so a tag table we cannot decrypt must NOT abort the whole
/// recovery — it degrades to no tag names (the recovered entries simply lose their tags). Fixing
/// the asymmetry at the source (rewrap tags too) is a `rewrap_snapshots` follow-up.
async fn read_snapshot_tag_names(
    session: &VaultSession,
    repo: &SqliteVaultRepository,
) -> HashMap<TagId, String> {
    let mut map: HashMap<TagId, String> = HashMap::new();
    let Ok(tag_rows) = repo.all_tags().await else {
        return map;
    };
    for tag_row in tag_rows {
        let Ok(aad) = tag_aad(&tag_row.id) else {
            continue;
        };
        let Ok(plaintext) = session.crypto.decrypt_tag(
            session.kek.expose(),
            &tag_row.nonce,
            &tag_row.ciphertext,
            &aad,
        ) else {
            // All tags share the snapshot-moment KEK, so one failure means the whole table is
            // under an old KEK (a pre-credential-change snapshot the rewrap didn't cover). Warn
            // once and recover the entries WITHOUT tag names rather than refusing everything.
            tracing::warn!(
                "snapshot tags predate a credential change and can't be resolved under the \
                 current KEK; recovering entries without their tag names"
            );
            return HashMap::new();
        };
        if let Ok(payload) = serde_json::from_slice::<TagPayload>(&plaintext) {
            map.insert(tag_row.id.clone(), payload.name);
        }
    }
    map
}

/// Read + decrypt a document's blob from the snapshot object pool. The pooled file is
/// `[24-byte nonce][ciphertext + tag]` (the same on-disk shape as a live blob); split the
/// nonce, verify it matches the payload's `blob_nonce` (the binding authority), and decrypt
/// under the entry DEK + `blob_aad`. `None` when the manifest records no object for this
/// entry (a document with no blob — tolerated, not fatal).
fn read_snapshot_blob(
    manifest: &SnapshotManifest,
    snapshots_dir: &Path,
    entry_id: &EntryId,
    crypto: &dyn CryptoProvider,
    dek: &[u8; DEK_LEN],
    blob_nonce: &[u8; NONCE_LEN],
) -> Result<Option<Zeroizing<Vec<u8>>>, VaultError> {
    let Some(obj) = manifest
        .objects
        .iter()
        .find(|o| o.entry_id == entry_id.as_str())
    else {
        return Ok(None);
    };
    let path = store::object_path(snapshots_dir, &obj.blake3);
    let bytes = std::fs::read(&path)
        .map_err(|e| VaultError::Storage(StorageError::Io(format!("read snapshot object: {e}"))))?;
    // Split [nonce || ciphertext]; a too-short object is indistinguishable from tampering.
    let disk_nonce = bytes.get(..NONCE_LEN).ok_or(VaultError::DecryptionFailed)?;
    let ciphertext = bytes.get(NONCE_LEN..).ok_or(VaultError::DecryptionFailed)?;
    if disk_nonce != blob_nonce.as_slice() {
        return Err(VaultError::DecryptionFailed);
    }
    let aad = blob_aad(entry_id)?;
    let plaintext = crypto.decrypt_entry(dek, blob_nonce, ciphertext, &aad)?;
    Ok(Some(plaintext))
}

/// Build the staging session (preview + serialized bundle) from a ready list of
/// `ExportEntry` + document plaintexts — the shared tail of the snapshot source. Every entry
/// is committable (1:1 `row_id` ↔ bundle index); a child whose parent folder is absent from
/// the batch is badged "will import flat" (④), and near-duplicates get the advisory hint (②).
fn stage_from_entries(
    entries: Vec<ExportEntry>,
    blobs: Vec<(String, Zeroizing<Vec<u8>>)>,
) -> Result<ImportSession, VaultError> {
    let folder_ids: HashSet<String> = entries
        .iter()
        .filter(|e| matches!(e, ExportEntry::Folder(_)))
        .filter_map(|e| e.id().map(str::to_owned))
        .collect();

    let mut preview = Vec::with_capacity(entries.len());
    let mut entry_of = Vec::with_capacity(entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let row_id = u32::try_from(i).unwrap_or(u32::MAX);
        let mut r = preview_row(row_id, entry);
        if let Some(fid) = source_folder_id(entry) {
            if !folder_ids.contains(fid) {
                r.status = flat_folder_warning();
            }
        }
        preview.push(r);
        entry_of.push(Some(row_id));
    }

    apply_dedup(&mut preview);
    let bundle_json = Zeroizing::new(
        serde_json::to_vec(&ExportBundle::new(entries))
            .map_err(|e| VaultError::MalformedPayload(format!("snapshot bundle: {e}")))?,
    );
    Ok(ImportSession {
        bundle_json,
        blobs,
        preview,
        entry_of,
    })
}

/// A wrong/stale KEK makes AEAD decryption fail; surface that as `SnapshotStale` so the
/// shell can ask for the snapshot's own password rather than reporting generic corruption.
fn snapshot_kek_error(err: VaultError) -> VaultError {
    match err {
        VaultError::DecryptionFailed => VaultError::SnapshotStale,
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Preview derivation (derivatives only).
// ---------------------------------------------------------------------------

fn preview_row(row_id: u32, entry: &ExportEntry) -> ImportPreviewRow {
    let mk = |entry_type: &str, meta: &ExportMeta, username: Option<String>, has_password: bool| {
        ImportPreviewRow {
            row_id,
            entry_type: entry_type.to_owned(),
            name: meta.name.clone(),
            username,
            url: meta.url.clone(),
            tags: meta.tags.clone(),
            has_password,
            status: RowStatus::Ok,
            duplicate_of: None,
        }
    };
    match entry {
        ExportEntry::Login(e) => mk(
            "login",
            &e.meta,
            Some(e.username.clone()),
            !e.password.expose_secret().is_empty(),
        ),
        ExportEntry::Card(e) => mk("card", &e.meta, None, true),
        ExportEntry::SshKey(e) => mk("ssh_key", &e.meta, None, true),
        ExportEntry::ApiKey(e) => mk("api_key", &e.meta, None, true),
        ExportEntry::EnvVars(e) => mk("env_vars", &e.meta, None, !e.vars.is_empty()),
        ExportEntry::Note(e) => mk("note", &e.meta, None, true),
        ExportEntry::Document(e) => mk("document", &e.meta, None, false),
        ExportEntry::Identity(e) => mk("identity", &e.meta, None, e.national_id.is_some()),
        ExportEntry::Folder(e) => mk("folder", &e.meta, None, false),
        ExportEntry::Unknown(e) => {
            let name = e
                .raw
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("(unknown entry)")
                .to_owned();
            ImportPreviewRow {
                row_id,
                entry_type: "unknown".to_owned(),
                name,
                username: None,
                url: None,
                tags: Vec::new(),
                has_password: false,
                status: RowStatus::Ok,
                duplicate_of: None,
            }
        }
    }
}

/// The source parent-folder id of an entry (a batch-local ULID), if any.
fn source_folder_id(entry: &ExportEntry) -> Option<&str> {
    match entry {
        ExportEntry::Login(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Card(e) => e.meta.folder_id.as_deref(),
        ExportEntry::SshKey(e) => e.meta.folder_id.as_deref(),
        ExportEntry::ApiKey(e) => e.meta.folder_id.as_deref(),
        ExportEntry::EnvVars(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Note(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Document(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Identity(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Folder(e) => e.meta.folder_id.as_deref(),
        ExportEntry::Unknown(e) => e.raw.get("folder_id").and_then(serde_json::Value::as_str),
    }
}

fn flat_folder_warning() -> RowStatus {
    RowStatus::Warning("its folder is not in this file — will import at the top level".to_owned())
}

// ---------------------------------------------------------------------------
// Duplicate detection (Decision ②, url + psl). Advisory only — never merges.
// ---------------------------------------------------------------------------

/// Normalized comparison key: `scheme://host[:non-default-port]`, lowercased. The
/// `url` crate already drops default ports and normalizes the host; a scheme-less
/// input is treated as `https`. Unparseable → `None` (no dedup).
fn normalize_host(raw: &str) -> Option<String> {
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let url = url::Url::parse(&with_scheme).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    Some(url.port().map_or_else(
        || format!("{}://{host}", url.scheme()),
        |p| format!("{}://{host}:{p}", url.scheme()),
    ))
}

/// The registrable eTLD+1 of a URL's host, via the compile-time Public Suffix List
/// (`psl`) — IDN/punycode aware. Used only for the soft "shares domain" hint.
fn base_domain(raw: &str) -> Option<String> {
    let with_scheme = if raw.contains("://") {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let url = url::Url::parse(&with_scheme).ok()?;
    let host = url.host_str()?;
    psl::domain_str(host).map(str::to_owned)
}

fn apply_dedup(rows: &mut [ImportPreviewRow]) {
    // Per-row keys computed up front to sidestep the borrow while mutating.
    let host_keys: Vec<Option<String>> = rows
        .iter()
        .map(|r| r.url.as_deref().and_then(normalize_host))
        .collect();
    let base_keys: Vec<Option<String>> = rows
        .iter()
        .map(|r| r.url.as_deref().and_then(base_domain))
        .collect();

    // Base-domain counts (for the soft "shares domain" hint).
    let mut base_counts: HashMap<&str, u32> = HashMap::new();
    for key in base_keys.iter().flatten() {
        let c = base_counts.entry(key.as_str()).or_insert(0);
        *c = c.saturating_add(1);
    }

    let mut first_host: HashMap<String, u32> = HashMap::new();
    for (i, row) in rows.iter_mut().enumerate() {
        if let Some(Some(key)) = host_keys.get(i) {
            match first_host.get(key) {
                // An exact-host match earlier in the batch → flag as a likely dup.
                Some(&first_id) => row.duplicate_of = Some(first_id),
                None => {
                    first_host.insert(key.clone(), row.row_id);
                }
            }
        }
        // A base-domain sibling but not an exact duplicate → a soft near-dup hint,
        // and only if nothing more important already occupies the badge.
        if row.duplicate_of.is_none() && matches!(row.status, RowStatus::Ok) {
            if let Some(Some(base)) = base_keys.get(i) {
                if base_counts.get(base.as_str()).copied().unwrap_or(0) > 1 {
                    row.status =
                        RowStatus::Warning(format!("shares the domain {base} with another row"));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// commit_import — write the picked rows into the live vault.
// ---------------------------------------------------------------------------

/// Commit the picked preview rows into the vault.
///
/// Best-effort per row (a failed row is reported, not fatal — Decision A / the CSV
/// row-badge model). Folders are created first so `folder_id` can be remapped
/// through the batch (Decision ④); tags are matched-or-created by name; documents
/// re-encrypt under fresh DEKs; `Unknown` entries are written byte-faithfully.
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn commit_import(
    session: &mut VaultSession,
    actions: &[ImportAction],
) -> Result<ImportReport, VaultError> {
    // Take the staging out of the session (leaving it empty) so `&mut session` is
    // free for the write path below. The staging drops — zeroizing — at fn end.
    let staging = session.import.take().ok_or(VaultError::ImportNotStarted)?;

    let bundle: ExportBundle = serde_json::from_slice(&staging.bundle_json)
        .map_err(|e| VaultError::MalformedPayload(format!("staged bundle: {e}")))?;
    let mut entries: Vec<Option<ExportEntry>> = bundle.entries.into_iter().map(Some).collect();

    // Resolve picked preview rows → (row_id, bundle index). Uncommittable rows
    // (entry_of == None) and Skips are dropped here.
    let mut picks: Vec<Pick> = Vec::new();
    let mut skipped: u64 = 0;
    for a in actions {
        match a.action {
            RowAction::Skip => skipped = skipped.saturating_add(1),
            RowAction::Import => {
                if let Some(Some(idx)) = staging.entry_of.get(a.row_id as usize) {
                    picks.push((a.row_id, *idx as usize));
                }
            }
        }
    }

    let mut imported: u64 = 0;
    let mut failed: Vec<ImportFailure> = Vec::new();
    let mut folder_map: HashMap<String, EntryId> = HashMap::new();

    // The set of PICKED folder source-ids — a parent must be picked (not merely
    // present) to nest under it; otherwise the child imports flat.
    let picked_folder_ids: HashSet<String> = picks
        .iter()
        .filter_map(|&(_, idx)| {
            entries
                .get(idx)
                .and_then(|o| o.as_ref())
                .filter(|e| matches!(e, ExportEntry::Folder(_)))
                .and_then(|e| e.id().map(str::to_owned))
        })
        .collect();

    let (folder_picks, other_picks): (Vec<Pick>, Vec<Pick>) =
        picks.iter().copied().partition(|&(_, idx)| {
            matches!(
                entries.get(idx).and_then(|o| o.as_ref()),
                Some(ExportEntry::Folder(_))
            )
        });

    // 1. Folders first, in dependency order (parent before child).
    commit_folders(
        session,
        &mut entries,
        &folder_picks,
        &picked_folder_ids,
        &mut folder_map,
        &mut imported,
        &mut failed,
    )
    .await;

    // 2. Everything else, remapping folder_id through the batch.
    for &(row_id, idx) in &other_picks {
        let Some(entry) = entries.get_mut(idx).and_then(Option::take) else {
            continue;
        };
        match commit_one(session, entry, &folder_map, &staging.blobs).await {
            Ok(_) => imported = imported.saturating_add(1),
            Err(e) => failed.push(ImportFailure {
                row_id,
                message: e.to_string(),
            }),
        }
    }

    Ok(ImportReport {
        imported,
        skipped,
        failed,
    })
}

#[allow(clippy::too_many_arguments)] // a focused private step; splitting state would obscure it
async fn commit_folders(
    session: &mut VaultSession,
    entries: &mut [Option<ExportEntry>],
    folder_picks: &[Pick],
    picked_folder_ids: &HashSet<String>,
    folder_map: &mut HashMap<String, EntryId>,
    imported: &mut u64,
    failed: &mut Vec<ImportFailure>,
) {
    let mut pending: Vec<Pick> = folder_picks.to_vec();
    loop {
        let before = pending.len();
        let mut still: Vec<Pick> = Vec::new();
        for (row_id, idx) in std::mem::take(&mut pending) {
            let parent = entries
                .get(idx)
                .and_then(|o| o.as_ref())
                .and_then(source_folder_id)
                .map(str::to_owned);
            // Ready when the parent is None, absent from the picks (→ flat), or
            // already created. A no-progress pass is handled below (forced flat).
            let ready = parent
                .as_ref()
                .is_none_or(|p| !picked_folder_ids.contains(p) || folder_map.contains_key(p));
            if !ready {
                still.push((row_id, idx));
                continue;
            }
            commit_folder(session, entries, row_id, idx, folder_map, imported, failed).await;
        }

        // `still` holds only the not-yet-ready folders; retry until drained.
        if still.is_empty() {
            break;
        }
        // No progress this pass → break the cycle by forcing the remainder flat.
        if still.len() == before {
            for (row_id, idx) in std::mem::take(&mut still) {
                commit_folder(session, entries, row_id, idx, folder_map, imported, failed).await;
            }
            break;
        }
        pending = still;
    }
}

#[allow(clippy::too_many_arguments)] // one folder's commit; the state it threads is inherent
async fn commit_folder(
    session: &mut VaultSession,
    entries: &mut [Option<ExportEntry>],
    row_id: u32,
    idx: usize,
    folder_map: &mut HashMap<String, EntryId>,
    imported: &mut u64,
    failed: &mut Vec<ImportFailure>,
) {
    let Some(entry) = entries.get_mut(idx).and_then(Option::take) else {
        return;
    };
    let source_id = entry.id().map(str::to_owned);
    match commit_one(session, entry, folder_map, &[]).await {
        Ok(new_id) => {
            *imported = imported.saturating_add(1);
            if let Some(src) = source_id {
                folder_map.insert(src, new_id); // so children remap onto the fresh ULID
            }
        }
        Err(e) => failed.push(ImportFailure {
            row_id,
            message: e.to_string(),
        }),
    }
}

/// Commit exactly one entry via the write path that fits it, returning its fresh
/// `EntryId`: standard entries → `create_entry`; documents → `import_document`;
/// `Unknown` → the low-level write.
async fn commit_one(
    session: &mut VaultSession,
    entry: ExportEntry,
    folder_map: &HashMap<String, EntryId>,
    blobs: &[(String, Zeroizing<Vec<u8>>)],
) -> Result<EntryId, VaultError> {
    // Unknown carries its meta inside `raw`; the others carry an ExportMeta.
    if matches!(entry, ExportEntry::Unknown(_)) {
        return commit_unknown(session, entry, folder_map).await;
    }

    let export_meta = entry_meta(&entry)
        .ok_or_else(|| VaultError::MalformedPayload("unexpected unknown entry".to_owned()))?
        .clone();
    let entry_type = export_entry_type(&entry)
        .ok_or_else(|| VaultError::MalformedPayload("unexpected unknown entry".to_owned()))?;
    let meta = resolve_meta(session, &export_meta, entry_type, folder_map).await?;

    let source_id = export_meta.id.clone();
    match export_to_payload(entry, meta) {
        ImportedEntry::Standard(payload) => {
            let out = create_entry(session, CreateEntryInput { payload }).await?;
            Ok(out.entry_id)
        }
        ImportedEntry::Document(doc) => commit_document(session, doc, blobs, &source_id).await,
        // Unreachable: a known entry never maps to Unknown.
        ImportedEntry::Unknown(_) => Err(VaultError::MalformedPayload(
            "unexpected unknown mapping".to_owned(),
        )),
    }
}

async fn commit_document(
    session: &mut VaultSession,
    doc: ImportedDocument,
    blobs: &[(String, Zeroizing<Vec<u8>>)],
    source_id: &str,
) -> Result<EntryId, VaultError> {
    let content = blobs
        .iter()
        .find(|(id, _)| id == source_id)
        .map(|(_, bytes)| bytes.to_vec())
        .ok_or_else(|| {
            VaultError::MalformedPayload(format!("document blob missing for {source_id}"))
        })?;
    import_document(
        session,
        ImportDocumentInput {
            filename: doc.filename,
            mime_type: doc.mime_type,
            content,
            meta: doc.meta,
        },
    )
    .await
}

async fn commit_unknown(
    session: &mut VaultSession,
    entry: ExportEntry,
    folder_map: &HashMap<String, EntryId>,
) -> Result<EntryId, VaultError> {
    // Recover the source parent folder from the raw, then remap it through the batch.
    let remapped = source_folder_id(&entry).and_then(|src| folder_map.get(src).cloned());
    // `export_to_payload` only reads `meta.folder_id` for an Unknown.
    let mut meta = CommonMeta::new("", EntryType::Unknown(String::new()));
    meta.folder_id = remapped;
    match export_to_payload(entry, meta) {
        ImportedEntry::Unknown(raw) => commit_unknown_raw(session, &raw).await,
        // Unreachable: an Unknown export maps to an Unknown import.
        ImportedEntry::Standard(_) | ImportedEntry::Document(_) => Err(
            VaultError::MalformedPayload("unexpected known mapping".to_owned()),
        ),
    }
}

/// The low-level write for an `Unknown` payload — `create_entry` and
/// `EntryPayload::to_encryptable_json` both refuse it. Mirrors `create_entry`'s
/// crypto discipline (fresh ULID + DEK + nonce, one `Created` audit) but persists
/// the byte-faithful `raw` JSON directly (Decision ③). Validation is skipped
/// deliberately: `folder_id` is already remapped-valid or `None`, and any
/// unresolvable source tag ids in `raw` are harmless (they simply don't render).
async fn commit_unknown_raw(
    session: &mut VaultSession,
    raw: &serde_json::Value,
) -> Result<EntryId, VaultError> {
    let entry_id = EntryId::new();
    let version: i64 = 1;
    let when = now();

    let payload_bytes = Zeroizing::new(
        serde_json::to_vec(raw).map_err(|e| VaultError::MalformedPayload(e.to_string()))?,
    );
    let dek = session.crypto.generate_dek();
    let aad = entry_aad(&entry_id, version)?;
    let (nonce, ciphertext) = session.crypto.encrypt_entry(&dek, &payload_bytes, &aad)?;
    let dek_wrapped = session.crypto.wrap_dek(&dek, session.kek.expose())?;

    let row = EntryRow {
        id: entry_id.clone(),
        version,
        cipher_suite: session.config.preferred_cipher_suite,
        dek_wrapped,
        nonce,
        ciphertext,
        created_at: when,
        updated_at: when,
        accessed_at: None,
        is_trashed: false,
        trashed_at: None,
    };
    session.repo.insert_entry(&row).await?;
    append_audit(session, AuditAction::Created, Some(&entry_id)).await?;

    // Index projection: reuse the at-rest deserializer, which routes an unknown
    // entry_type back into `EntryPayload::Unknown` (no re-serialize of Unknown).
    let payload = EntryPayload::from_decrypted_json(&payload_bytes)?;
    let index_entry = IndexEntry::from_payload(&payload, &row);
    session.index.insert_entry(index_entry);
    Ok(entry_id)
}

/// Build a resolved [`CommonMeta`] from an [`ExportMeta`]: tags matched-or-created
/// by name (Decision ④); `folder_id` remapped through the batch, or `None` (flat)
/// when the source folder is absent.
async fn resolve_meta(
    session: &mut VaultSession,
    export_meta: &ExportMeta,
    entry_type: EntryType,
    folder_map: &HashMap<String, EntryId>,
) -> Result<CommonMeta, VaultError> {
    let mut tag_ids: Vec<TagId> = Vec::with_capacity(export_meta.tags.len());
    for name in &export_meta.tags {
        tag_ids.push(create_tag(session, name, None).await?);
    }
    let folder_id = export_meta
        .folder_id
        .as_deref()
        .and_then(|src| folder_map.get(src).cloned());
    Ok(common_meta_from_export(
        export_meta,
        entry_type,
        tag_ids,
        folder_id,
    ))
}

/// The [`ExportMeta`] of a non-`Unknown` entry (`None` for `Unknown`, whose meta
/// lives inside `raw`). `commit_one` routes `Unknown` away before calling this.
const fn entry_meta(entry: &ExportEntry) -> Option<&ExportMeta> {
    Some(match entry {
        ExportEntry::Login(e) => &e.meta,
        ExportEntry::Card(e) => &e.meta,
        ExportEntry::SshKey(e) => &e.meta,
        ExportEntry::ApiKey(e) => &e.meta,
        ExportEntry::EnvVars(e) => &e.meta,
        ExportEntry::Note(e) => &e.meta,
        ExportEntry::Document(e) => &e.meta,
        ExportEntry::Identity(e) => &e.meta,
        ExportEntry::Folder(e) => &e.meta,
        ExportEntry::Unknown(_) => return None,
    })
}
