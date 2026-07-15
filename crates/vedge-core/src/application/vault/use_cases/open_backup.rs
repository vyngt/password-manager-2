//! Materialise a `.vbk` as a vault at a path that has **nothing there** (slice 5.2.2) —
//! the SPACE half of Decision ⑦, and the everyday verb.
//!
//! **The `.vbk` is not a time machine. It is a vault in a suitcase.** This is how a vault
//! gets onto a new machine, how a second copy gets made, and how a backup gets looked at
//! without risking the original.
//!
//! # 🔴 It cannot overwrite anything. Full stop.
//!
//! An occupied destination is [`VaultError::DestinationOccupied`] — no flag, no confirmation,
//! no override. Overwriting is a **different operation, with a different name and a different
//! button** ([`super::replace_vault`]).
//!
//! This is not defensive politeness; it is the entire design. 5.2 needed **four**
//! confirmation dialogs to make "restore a `.vbk` over a vault" survivable, and **three of
//! them existed only because an import was pointed at an existing file**:
//!
//! - the uuid-match guard — 🟢 gone. There is no target to mismatch.
//! - **H2**, an unreadable target silently disarming that guard — 🟢 gone. No target identity
//!   is read, so there is nothing to disarm. *H2 was an artifact of the wrong boundary, not a
//!   bug in the code.*
//! - the rollback gate — 🟢 gone. Opening a backup rolls nothing back; nothing is replaced.
//!
//! The moment a *"just this once"* override appears next to `DestinationOccupied`, that
//! boundary is gone and all three come back. Do not add one.
//!
//! # ② A copy beside a live original MUST get a fresh `vault_uuid`
//!
//! `02 Architecture/13 - Vault Identity` anticipated this: *"a copy of a vault is the same
//! vault (same uuid) — a future 'duplicate as a new vault' feature must mint a fresh one."*
//! **This is that feature.**
//!
//! It is not bookkeeping. The keychain rollback baseline is keyed `counter:{uuid}` (5.2c), so
//! two divergent files sharing one uuid **fight over one baseline**: open the copy → the
//! mirror advances; open the original → `file.commit_counter < baseline` → 🔴 a false rollback
//! warning on every single unlock, forever. The security alarm starts crying wolf and the user
//! learns to ignore it.
//!
//! So: if another **registered, openable** vault on this machine already carries the backup's
//! uuid, this is a *duplicate* → mint a fresh uuid into the copy, and carry the Secret Key
//! across so it still opens with the master password alone. If nothing else carries it (a new
//! machine; the original deleted or corrupt), this is the *same vault, relocated* → keep the
//! uuid, and the keychain-less unlock falls through to 5.1's Emergency Kit panel exactly as
//! designed.

use std::path::{Path, PathBuf};

use tracing::{instrument, warn};

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::shared::{
    BLOBS_DIR, SNAPSHOTS_DIR, StorageError, VAULT_FILE, format_rfc3339_millis, now,
};
use crate::domain::vault::entities::CURRENT_SCHEMA_VERSION;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::manifest::{
    BACKUP_FORMAT_VERSION, BLOBS_PREFIX, BackupManifest, VAULT_MEMBER,
};
use crate::infrastructure::backup::target::{TargetIdentity, TargetState, read_target_state};
use crate::infrastructure::backup::{archive, journal};
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use crate::application::vault::ports::repository::VaultRepository;

/// Which archive to open, and where to put it.
#[derive(Debug, Clone)]
pub struct OpenBackupInput {
    pub archive_path: PathBuf,
    /// The vault HOME to create (`<name>.vedge/`). 🔴 **MUST NOT EXIST.**
    pub dest_home: PathBuf,
    /// Every vault home this machine knows about, for the ② duplicate check. Supplied by the
    /// shell from `app.db`'s recents — the application layer does not reach into the app DB.
    pub known_vaults: Vec<PathBuf>,
}

/// What opening produced — non-invertible facts only.
#[derive(Debug, Clone)]
pub struct OpenBackupReport {
    pub home: PathBuf,
    /// The uuid the opened vault ended up with: the backup's, or a freshly minted one.
    pub vault_uuid: Option<String>,
    /// ②: the live vault this was recognised as a copy OF, if any. `Some` ⇔ `fresh_uuid`.
    pub duplicate_of: Option<PathBuf>,
    /// ②: a fresh `vault_uuid` was minted because this is a duplicate.
    pub fresh_uuid: bool,
    /// The source vault's Secret Key was copied to the new uuid, so the copy opens with the
    /// master password alone. `false` ⇒ this machine's keychain had none ⇒ the Emergency Kit
    /// is needed, which is correct — not an error.
    pub secret_key_copied: bool,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub opened_at: String,
}

/// A staged vault home, ready to become live.
pub(super) struct StagedHome {
    /// BLAKE3 of the staged `vault.vdb` — anchors the journal on the Replace path.
    pub vault_blake3: String,
    pub blob_count: u64,
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Extract an archive into `staging_dir` as a complete vault HOME, verifying as it goes.
///
/// Shared by [`open_backup`] and [`super::replace_vault::replace_vault_from_backup`] — the
/// two verbs differ entirely in what they do with the result (a fresh path vs. a journaled
/// swap), and not at all in how the bytes come out of the tar.
///
/// Guarantees on return: `vault.vdb` extracted, re-hashed against the manifest and fsynced;
/// `blobs/` **and** `snapshots/` present (the fail-closed blob store refuses a home with no
/// `blobs/`, even when the backup carried none); every blob member checked against
/// [`journal::is_safe_blob_name`] — a `.vbk` is an arbitrary user-chosen file, so a crafted
/// member like `blobs/../../evil` must not escape. On any failure the staging dir is removed.
pub(super) fn stage_home_from_archive(
    archive_path: &Path,
    manifest: &BackupManifest,
    staging_dir: &Path,
) -> Result<StagedHome, VaultError> {
    if staging_dir.exists() {
        // A stale staging dir from an aborted attempt with no journal — reap it.
        std::fs::remove_dir_all(staging_dir).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(staging_dir).map_err(|e| io_ctx("create staging", &e))?;

    // Run the body, and tear the staging dir down on ANY failure — a half-extracted home must
    // never be left where a caller (or a later run) could mistake it for a real one.
    let staged = stage_inner(archive_path, manifest, staging_dir);
    if staged.is_err() {
        std::fs::remove_dir_all(staging_dir).ok();
    }
    staged
}

fn stage_inner(
    archive_path: &Path,
    manifest: &BackupManifest,
    staging_dir: &Path,
) -> Result<StagedHome, VaultError> {
    let staged_vault = staging_dir.join(VAULT_FILE);
    archive::extract_member(archive_path, VAULT_MEMBER, &staged_vault)?;
    journal::fsync_file(&staged_vault)?;

    // The staging dir IS the new vault home (slice 5.2.0): guarantee its fixed members exist
    // even for a no-blob backup, so the restored home is complete.
    let staged_blobs = staging_dir.join(BLOBS_DIR);
    std::fs::create_dir_all(&staged_blobs).map_err(|e| io_ctx("create staged blobs", &e))?;
    std::fs::create_dir_all(staging_dir.join(SNAPSHOTS_DIR))
        .map_err(|e| io_ctx("create staged snapshots", &e))?;

    let mut blob_count: u64 = 0;
    for file in &manifest.files {
        let Some(rel) = file.name.strip_prefix(BLOBS_PREFIX) else {
            continue; // the vault member; handled above
        };
        if !journal::is_safe_blob_name(rel) {
            return Err(io_msg(format!(
                "backup contains an unsafe blob member name: {}",
                file.name
            )));
        }
        let dest = staged_blobs.join(rel);
        archive::extract_member(archive_path, &file.name, &dest)?;
        journal::fsync_file(&dest)?;
        blob_count = blob_count.saturating_add(1);
    }
    journal::fsync_dir(staging_dir);

    // Re-hash the extracted vault against the manifest. On the Replace path this value anchors
    // the journal and is what crash recovery re-verifies before trusting the staged snapshot.
    let expected = manifest
        .files
        .iter()
        .find(|m| m.name == VAULT_MEMBER)
        .map(|m| m.blake3.clone())
        .ok_or_else(|| io_msg("manifest is missing the vault member"))?;
    let (_size, vault_blake3) = archive::hash_file(&staged_vault)?;
    if vault_blake3 != expected {
        return Err(io_msg("staged vault hash mismatch after extraction"));
    }

    Ok(StagedHome {
        vault_blake3,
        blob_count,
    })
}

/// ②: is another **registered, openable** vault on this machine already carrying `uuid`?
///
/// Returns the vault AND the identity we read from it — the caller almost always wants both
/// (`inspect_backup` needs its `verify_hash_prefix` for M3's credential compare), and throwing
/// it away would mean opening the same database twice.
///
/// A **corrupt** vault carrying the uuid does not count — you are recovering from that one, not
/// duplicating it, and it must keep its identity. Likewise a recents row whose folder is gone.
///
/// No need to exclude `dest_home` from the scan: [`open_backup`] has already refused if anything
/// exists there, so a not-yet-created path cannot be a readable vault.
///
/// 🔴 Shared with [`super::inspect_backup`] on purpose. The preview *promises* what this use case
/// will decide ("this will become a copy, with an identity of its own"), so the two must run the
/// **same** scan. Two copies of it would be free to drift, and the thing they would drift about
/// is whether a user's original vault starts crying wolf on every unlock, forever.
pub(super) async fn find_duplicate(
    known_vaults: &[PathBuf],
    uuid: Option<&str>,
) -> Option<(PathBuf, Box<TargetIdentity>)> {
    let uuid = uuid?;
    for home in known_vaults {
        if let TargetState::Readable(id) = read_target_state(home).await
            && id.vault_uuid.as_deref() == Some(uuid)
        {
            return Some((home.clone(), id));
        }
    }
    None
}

/// ②: mint a fresh `vault_uuid` into the STAGED vault, before it is ever registered.
///
/// `vault_uuid` is a plaintext `vault_config` column — no KEK, no master password. Returns the
/// new uuid.
///
/// 🔴 The connection is closed EXPLICITLY before returning, because the very next thing the
/// caller does is rename this directory, and Windows will not rename a directory holding an
/// open `.vdb` handle. sqlx's pool close is async-on-`Drop`, so an explicit `close().await` is
/// the only guarantee — the 5.2.1 field bug in one line: *an async close is not a close.*
async fn mint_fresh_uuid(staged_home: &Path) -> Result<String, VaultError> {
    let fresh = ulid::Ulid::new().to_string();
    let db = VaultDbConnection::open(&staged_home.join(VAULT_FILE)).await?;
    let repo = SqliteVaultRepository::new(db.handle());

    let result = async {
        let mut cfg = repo.load_config().await?;
        cfg.vault_uuid = Some(fresh.clone());
        repo.save_config(&cfg).await
    }
    .await;

    drop(repo); // release the handle clone so `close()` can actually close the pool
    db.close().await?;
    result?;
    Ok(fresh)
}

#[instrument(skip_all, fields(dest = %input.dest_home.display()))]
pub async fn open_backup(
    keychain: &dyn KeychainProvider,
    input: OpenBackupInput,
) -> Result<OpenBackupReport, VaultError> {
    // ---- PRE: verify + refuse. Nothing is created until every check has passed. ----
    // 1. Integrity gate: per-member hashes + the `.blake3` sidecar when present (L2).
    let manifest = archive::verify_archive(&input.archive_path)?;

    // 2. 🔴 H1: refuse NEWER, accept OLDER. An older `.vbk` must open forever.
    if manifest.format_version > BACKUP_FORMAT_VERSION {
        return Err(VaultError::BackupUnsupportedFormat(manifest.format_version));
    }
    if manifest.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(VaultError::UnsupportedSchemaVersion(
            manifest.schema_version,
        ));
    }

    // 3. 🔴 THE constraint. Not "is it a vault?" — is there ANYTHING here? A file, an empty
    //    folder, someone's photos: if it exists, we do not touch it. No flag, no override.
    if input.dest_home.exists() {
        return Err(VaultError::DestinationOccupied(
            input.dest_home.display().to_string(),
        ));
    }
    let parent = input
        .dest_home
        .parent()
        .ok_or_else(|| io_msg("the destination has no parent directory"))?;
    std::fs::create_dir_all(parent).map_err(|e| io_ctx("create destination parent", &e))?;

    // ---- STAGE: build the whole home beside its final path, on the same volume ----
    // Same-volume by construction (a sibling of the destination), so the commit below is an
    // atomic intra-volume rename. A crash before it leaves a dot-prefixed scrap and NO vault.
    let staging = staging_path(&input.dest_home)?;
    let staged = stage_home_from_archive(&input.archive_path, &manifest, &staging)?;

    // ---- ② duplicate detection, while it is still only a staged copy ----
    let duplicate_of = find_duplicate(&input.known_vaults, manifest.vault_uuid.as_deref())
        .await
        .map(|(home, _identity)| home);
    let (vault_uuid, fresh_uuid) = match duplicate_of {
        Some(_) => {
            // Mint BEFORE the commit: the copy must never be registered, unlocked, or
            // baselined under an identity it shares with a live vault — not even briefly.
            match mint_fresh_uuid(&staging).await {
                Ok(fresh) => (Some(fresh), true),
                Err(e) => {
                    // Refusing is the safe answer: proceeding would ship a duplicate identity
                    // and permanently poison BOTH vaults' rollback baselines (see module docs).
                    std::fs::remove_dir_all(&staging).ok();
                    return Err(e);
                }
            }
        }
        // Same vault, relocated (new machine / the original is gone or corrupt). Keep the uuid
        // — changing it here would orphan this vault from its own keychain entry and baseline.
        None => (manifest.vault_uuid.clone(), false),
    };

    // ---- COMMIT: one atomic rename. Nothing is destroyed, so no journal is needed. ----
    //
    // 🔴 On a blocking thread, with M1's retry — NOT a bare `std::fs::rename`.
    //
    // When this is a duplicate we have just opened (and closed) the staged `vault.vdb` to mint
    // its new uuid. `sqlx`'s pool close is not synchronous: for a short window afterwards
    // Windows still holds the `.vdb`/`-wal`/`-shm` handles and **refuses to rename the directory
    // containing them** (`ERROR_ACCESS_DENIED` 5 / `ERROR_SHARING_VIOLATION` 32). The handle does
    // release — it just needs a moment.
    //
    // Both halves matter. `rename_retrying` waits for it; `spawn_blocking` keeps the async
    // runtime free to actually FINISH the pool close while we wait. Retrying on the async thread
    // would starve the very task we are waiting for. (Measured before this fix: 4 of 6 opens
    // failed with os error 5. This is the same class of bug as the 5.2.1 field report — an async
    // close is not a close.)
    let (staging_c, dest_c) = (staging.clone(), input.dest_home.clone());
    let renamed =
        tokio::task::spawn_blocking(move || journal::rename_retrying(&staging_c, &dest_c))
            .await
            .map_err(|e| io_msg(format!("open-backup commit task join: {e}")))?;
    if let Err(e) = renamed {
        std::fs::remove_dir_all(&staging).ok();
        return Err(e);
    }
    journal::fsync_dir(parent);

    // ---- POST (L1): the vault exists and is valid. NOTHING below may fail the open. ----
    // ② Carry the Secret Key across to the new identity, so the copy opens with the master
    // password alone (the keychain is uuid-keyed since 5.2.0). Best-effort by design: if this
    // machine has no Secret Key for the source, the copy needs the Emergency Kit — which is
    // exactly right, and is 5.1's already-shipped flow, not a failure.
    let mut secret_key_copied = false;
    if fresh_uuid
        && let (Some(old), Some(new)) = (manifest.vault_uuid.as_deref(), vault_uuid.as_deref())
    {
        match keychain.read_secret_key(old) {
            Ok(sk) => match keychain.store_secret_key(new, &sk) {
                Ok(()) => secret_key_copied = true,
                Err(e) => {
                    warn!(error = %e, "opened vault: the Secret Key could not be stored under the new uuid — the Emergency Kit will be needed");
                }
            },
            Err(e) => {
                warn!(error = %e, "opened vault: no Secret Key for the source uuid on this machine — the Emergency Kit will be needed");
            }
        }
    }

    // 🔴 Deliberately NOT touched: `last_backup_at` (Decision ⑧ — opening is not backing up)
    // and the commit baseline (the first unlock's `rollback_check` establishes it for a fresh
    // uuid; seeding it here would only invent a number we have not verified).

    Ok(OpenBackupReport {
        home: input.dest_home,
        vault_uuid,
        duplicate_of,
        fresh_uuid,
        secret_key_copied,
        entry_count: manifest.entry_count,
        blob_count: staged.blob_count,
        opened_at: format_rfc3339_millis(now()),
    })
}

/// A hidden sibling of the destination — same volume, so the commit is an atomic rename.
fn staging_path(dest_home: &Path) -> Result<PathBuf, VaultError> {
    let parent = dest_home
        .parent()
        .ok_or_else(|| io_msg("the destination has no parent directory"))?;
    let stem = dest_home
        .file_name()
        .ok_or_else(|| io_msg("the destination has no name"))?
        .to_string_lossy()
        .into_owned();
    Ok(parent.join(format!(".{stem}.opening-tmp")))
}
