//! Live-safe backup of an unlocked vault to a single `.vbk` archive (slice 5.2).
//!
//! Snapshots the `.vdb` via `VACUUM INTO` (no forced lock), copies the home's
//! `blobs/` directory, and tars them with a per-file + whole-archive BLAKE3
//! manifest. The payload is already-encrypted ciphertext — no KEK, no DEK, no
//! plaintext ever touches this path (Decision ④). One `BackupCreated` audit row,
//! vault-level (`entry_id = None`), mirroring `export_emergency_kit`.
//!
//! 🔴 Decision ⑮-A: a `.vbk` must NEVER contain the `snapshots/` store, or a backup
//! would nest the snapshot store inside itself (exponential blowup). This holds by
//! construction — the ONLY filesystem read here is `session.blob.blob_dir()`
//! (`<home>/blobs`); `<home>/snapshots` is a sibling and is never enumerated. Any new
//! filesystem read on this path MUST preserve that exclusion.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{StorageError, format_rfc3339_millis, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive;
use crate::infrastructure::backup::manifest::{
    BACKUP_FORMAT_VERSION, BLOBS_PREFIX, BackupManifest, MANIFEST_NAME, ManifestFile, VAULT_MEMBER,
};
// The `verify_hash` prefix helper is the snapshot module's (slice 5.2.1); M3 reuses it verbatim
// so the two manifests describe credentials identically and a comparison can never mean two
// different things.
use crate::infrastructure::snapshot::manifest::verify_hash_prefix;

/// The vault's backup health (Decision ⑧, slice 5.2.2).
#[derive(Debug, Clone)]
pub struct BackupStatus {
    /// 🔴 When a `.vbk` was last written — **and only that**. Written by [`backup_vault`] alone.
    ///
    /// It is deliberately NOT derived from `BackupCreated` audit rows, because `create_snapshot`
    /// emits those too (5.2.1). A snapshot must never be able to silence *"you have never backed
    /// up"*: it lives on the same disk, in the same folder, and dies with it. Two different
    /// safety properties, two different columns, two different writers — so they cannot be
    /// confused by accident.
    pub last_backup_at: Option<crate::domain::shared::Timestamp>,
    /// When a local snapshot was last taken. Shown *beside* the above, never instead of it.
    pub last_snapshot_at: Option<crate::domain::shared::Timestamp>,
}

/// Read the vault's backup health. See [`BackupStatus`] for why these are two separate facts.
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn backup_status(session: &VaultSession) -> Result<BackupStatus, VaultError> {
    // The LIVE config: `session.config` is an unlock-time snapshot, and a backup taken during
    // this session would not be reflected in it.
    let cfg = session.repo.load_config().await?;
    Ok(BackupStatus {
        last_backup_at: cfg.last_backup_at,
        last_snapshot_at: cfg.last_snapshot_at,
    })
}

/// Where to write the archive. The `.blake3` sidecar goes beside it.
#[derive(Debug, Clone)]
pub struct BackupVaultInput {
    pub dest_archive: PathBuf,
}

/// The result surfaced to the UI (all non-invertible derivatives — counts,
/// hashes, timestamp; no key material, no plaintext).
#[derive(Debug, Clone)]
pub struct BackupReport {
    pub archive_path: PathBuf,
    pub archive_bytes: u64,
    pub archive_blake3: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub created_at: String,
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Append a literal suffix to a path (`x.vbk` + `.tmp` → `x.vbk.tmp`), unlike
/// `with_extension` which would *replace* the existing one.
fn append_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s: OsString = base.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// The staged blob members: manifest rows, `(tar-member-name, staged-path)` pairs, and count.
type StagedBlobs = (Vec<ManifestFile>, Vec<(String, PathBuf)>, u64);

/// Stage every `*.blob` from `blob_dir` into `blobs_staging`, returning the manifest rows and
/// the (member-name, staged-path) pairs for the tar plus the count. Skips cleanly if the dir
/// is absent. 🔴 Reads ONLY `blob_dir` (the home's `blobs/`) — never the `snapshots/` sibling
/// (Decision ⑮-A): a `.vbk` must not nest the snapshot store inside itself.
fn stage_blob_members(blob_dir: &Path, blobs_staging: &Path) -> Result<StagedBlobs, VaultError> {
    let mut files: Vec<ManifestFile> = Vec::new();
    let mut blob_members: Vec<(String, PathBuf)> = Vec::new();
    let mut blob_count: u64 = 0;

    let read_dir = match std::fs::read_dir(blob_dir) {
        Ok(rd) => Some(rd),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io_ctx("read blob dir", &e)),
    };
    if let Some(rd) = read_dir {
        let mut blob_files: Vec<PathBuf> = Vec::new();
        for entry in rd {
            let path = entry.map_err(|e| io_ctx("read blob entry", &e))?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("blob") {
                blob_files.push(path);
            }
        }
        blob_files.sort(); // deterministic archive layout
        if !blob_files.is_empty() {
            std::fs::create_dir_all(blobs_staging)
                .map_err(|e| io_ctx("create staging blobs dir", &e))?;
        }
        for src in blob_files {
            let file_name = src
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| io_msg("blob file has a non-UTF-8 name"))?
                .to_owned();
            let staged = blobs_staging.join(&file_name);
            std::fs::copy(&src, &staged).map_err(|e| io_ctx("copy blob", &e))?;
            let (size, blake3) = archive::hash_file(&staged)?;
            let member = format!("{BLOBS_PREFIX}{file_name}");
            files.push(ManifestFile {
                name: member.clone(),
                size,
                blake3,
            });
            blob_members.push((member, staged));
            blob_count = blob_count.saturating_add(1);
        }
    }
    Ok((files, blob_members, blob_count))
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn backup_vault(
    session: &VaultSession,
    input: BackupVaultInput,
) -> Result<BackupReport, VaultError> {
    let dest = input.dest_archive;
    let parent = dest
        .parent()
        .ok_or_else(|| io_msg("backup destination has no parent directory"))?;

    // Stage on the SAME volume as the destination — the archive lands by a single
    // intra-volume rename at the end. TempDir cleans up on drop (incl. on error).
    let staging =
        tempfile::TempDir::new_in(parent).map_err(|e| io_ctx("create staging dir", &e))?;
    let staging_path = staging.path();

    // 1. Live-safe snapshot of the vault DB (Decision ①). Dest must not exist →
    //    a fresh path inside the staging dir.
    let vault_snap = staging_path.join(VAULT_MEMBER);
    session.repo.vacuum_into(&vault_snap).await?;

    // 2. Hash the snapshot → its manifest row.
    let (vault_size, vault_hash) = archive::hash_file(&vault_snap)?;
    let mut files = vec![ManifestFile {
        name: VAULT_MEMBER.to_owned(),
        size: vault_size,
        blake3: vault_hash,
    }];

    // 3. Copy every `*.blob` from the home's `blobs/` DIRECTORY (source of truth, not the
    //    index — an orphan blob is still the user's bytes). 🔴 Decision ⑮-A: this is the
    //    ONLY vault-home read on the backup path; `snapshots/` is never enumerated.
    let blobs_staging = staging_path.join("blobs");
    let (blob_files, blob_members, blob_count) =
        stage_blob_members(session.blob.blob_dir(), &blobs_staging)?;
    files.extend(blob_files);

    // 4. Build + write the manifest.
    let created_at = format_rfc3339_millis(now());
    // M2: count ACTIVE entries only — `index.entries` includes trashed rows, so a
    // trashed entry would inflate the reported count. `all_active()` filters them.
    let entry_count = u64::try_from(session.index.all_active().len()).unwrap_or(u64::MAX);
    // `commit_counter` changes on every content write, so `session.config`'s
    // unlock-time snapshot is stale — read the LIVE value. Backup holds the session
    // (single writer), so this matches the `VACUUM INTO` snapshot's counter (5.2c).
    // The same live read gives us `verify_hash` for M3's credential prefix: a change_password
    // during this session would have rotated it, leaving `session.config`'s copy wrong — and a
    // WRONG prefix is worse than none, because it asserts a mismatch that isn't there.
    let live_config = session.repo.load_config().await?;
    let commit_counter = live_config.commit_counter;
    let verify_hash_prefix = Some(verify_hash_prefix(&live_config.verify_hash));
    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        vault_uuid: session.config.vault_uuid.clone(),
        schema_version: session.config.schema_version,
        created_at: created_at.clone(),
        entry_count,
        blob_count,
        commit_counter,
        verify_hash_prefix,
        files,
    };
    let manifest_path = staging_path.join(MANIFEST_NAME);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| VaultError::Storage(StorageError::Serialization(format!("manifest: {e}"))))?;
    std::fs::write(&manifest_path, &manifest_bytes).map_err(|e| io_ctx("write manifest", &e))?;

    // 5. Tar (manifest, vault, blobs…) → `dest.tmp`, hashing the stream.
    let mut members: Vec<(String, PathBuf)> =
        Vec::with_capacity(blob_members.len().saturating_add(2));
    members.push((MANIFEST_NAME.to_owned(), manifest_path));
    members.push((VAULT_MEMBER.to_owned(), vault_snap));
    members.extend(blob_members);

    let dest_tmp = append_suffix(&dest, ".tmp");
    let archive_blake3 = archive::write_archive(&members, &dest_tmp)?;

    // Stat the finished archive BEFORE the commit rename: a pre-commit read may fail the
    // operation; a post-commit one must not (L1). Sizes are identical across the rename.
    let archive_bytes = std::fs::metadata(&dest_tmp)
        .map_err(|e| io_ctx("stat staged archive", &e))?
        .len();

    // 6. COMMIT POINT: atomic move into place (intra-volume). On failure, clean up the
    //    temp so a reported-failed backup doesn't leave a stray `<dest>.tmp` behind.
    std::fs::rename(&dest_tmp, &dest).map_err(|e| {
        std::fs::remove_file(&dest_tmp).ok();
        io_ctx("rename archive into place", &e)
    })?;

    // ---- POST-COMMIT (L1): the archive is on disk and valid. NOTHING below may report a
    //      successful backup as failed — a best-effort failure is logged, never propagated. ----

    // 7. Advisory whole-archive hash sidecar (Decision ③). Now actually READ — as of
    //    slice 5.2.2 (finding L2), `archive::verify_archive` verifies against it when it
    //    is present. The path comes from `archive::sidecar_path` so the writer and the
    //    reader cannot drift apart.
    let sidecar = archive::sidecar_path(&dest);
    if let Err(e) = std::fs::write(&sidecar, format!("{archive_blake3}\n")) {
        tracing::warn!(error = %e, "backup committed but the .blake3 sidecar could not be written");
    }

    // 8. last_backup_at (Decision ⑧ — a `.vbk` is a real backup; SEPARATE from
    //    last_snapshot_at) + exactly one vault-level audit row (ciphertext left).
    if let Err(e) = session.repo.touch_last_backup_at(now()).await {
        tracing::warn!(error = %e, "backup committed but last_backup_at could not be recorded");
    }
    if let Err(e) =
        super::create_entry::append_audit(session, AuditAction::BackupCreated, None).await
    {
        tracing::warn!(error = %e, "backup committed but the audit row could not be written");
    }

    Ok(BackupReport {
        archive_path: dest,
        archive_bytes,
        archive_blake3,
        entry_count,
        blob_count,
        created_at,
    })
}
