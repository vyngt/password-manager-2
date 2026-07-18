//! Capture a local snapshot of an unlocked vault (slice 5.2.1) — the **TIME** half of
//! Decision ⑦, "Snapshot now".
//!
//! A snapshot is a directory in the home's `snapshots/` store: a live `VACUUM INTO` of the
//! `.vdb` plus the vault's blobs deduplicated into a content-addressed object pool (Decision
//! ⑪), described by a `manifest.json` written **last** and committed by an atomic rename.
//! Nothing but ciphertext is copied — no KEK, no DEK, no plaintext (like `backup_vault`).
//!
//! [`write_snapshot`] is the session-less core, shared with the pre-restore auto-snapshot
//! (Decision ⑭) taken by `revert_to_snapshot`. [`create_snapshot`] wraps it with the
//! session-only side effects: `last_snapshot_at` (🔴 NOT `last_backup_at` — Decision ⑧) and
//! one `BackupCreated` audit row (the `.vbk` variant is REUSED — no new `AuditAction`).

use std::path::{Path, PathBuf};

use tracing::instrument;

use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{StorageError, format_rfc3339_millis, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive;
use crate::infrastructure::snapshot::manifest::{
    SNAPSHOT_FORMAT_VERSION, SNAPSHOT_VAULT_FILE, SnapshotManifest, SnapshotObject, SnapshotReason,
    verify_hash_prefix,
};
use crate::infrastructure::snapshot::store;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

/// What a snapshot capture returns to the UI — non-invertible facts only.
#[derive(Debug, Clone)]
pub struct SnapshotReport {
    /// The snapshot's directory name, used as its stable id.
    pub id: String,
    /// RFC-3339 millis UTC (fixed-width).
    pub created_at: String,
    pub reason: SnapshotReason,
    pub entry_count: u64,
    pub blob_count: u64,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Null the Recovery Key slot in a freshly-`VACUUM INTO`'d snapshot copy (slice 5.7 ④, Fix A).
///
/// A snapshot has no recovery credential of its own — you *revert* it with the current
/// session KEK (`unlock_with_kek_quiet`), never recovery-unlock it. `VACUUM INTO` byte-copies
/// the live `recovery_slot`, so a snapshot captured while recovery is enrolled would carry a
/// live slot — and reverting to it would silently re-arm a Recovery Key the user may have
/// deliberately revoked. Null it at capture so snapshots are born slot-less. Runs BEFORE the
/// manifest hash (so `vault_blake3` matches the file that lands) and BEFORE the atomic rename.
///
/// Fails the snapshot on error (fail-safe — better no snapshot than one with a live slot); the
/// pre-restore auto-snapshot (⑭) treats that failure as best-effort at its own call site.
async fn null_recovery_slot(vault_db: &Path) -> Result<(), VaultError> {
    let db = VaultDbConnection::open(vault_db)
        .await
        .map_err(VaultError::Storage)?;
    let repo = SqliteVaultRepository::new(db.handle());
    repo.clear_recovery_slot().await?;
    drop(repo); // release the handle clone so `close()` can actually close the pool
    // Close so the WAL is checkpointed into the copy (self-contained) before it is hashed.
    db.close().await.map_err(VaultError::Storage)?;
    Ok(())
}

/// Session-less snapshot capture. Builds the snapshot dir + manifest and returns its report.
///
/// Writes NO DB rows and NO config timestamp — a caller that has a session
/// ([`create_snapshot`]) adds those; the pre-restore auto-snapshot (Decision ⑭) does not (its
/// side effects would land in a vault that is about to be swapped away). `entry_count` is
/// supplied by the caller: a session uses its cheap in-memory index, the revert path counts
/// from the DB (both ACTIVE-only — M2).
#[instrument(skip_all)]
pub(crate) async fn write_snapshot(
    snapshots_dir: &Path,
    blobs_dir: &Path,
    repo: &dyn VaultRepository,
    reason: SnapshotReason,
    entry_count: u64,
) -> Result<SnapshotReport, VaultError> {
    // Read the LIVE config (uuid / schema / verify_hash / commit_counter) — `commit_counter`
    // changes on every write, so an unlock-time snapshot would be stale; this matches the
    // `VACUUM INTO` we take just below (both against the same connection).
    let config = repo.load_config().await?;

    let now_ts = now();
    let created_at = format_rfc3339_millis(now_ts);

    std::fs::create_dir_all(snapshots_dir).map_err(|e| io_ctx("create snapshots dir", &e))?;

    // Derive the dir name from the timestamp; disambiguate the rare same-millisecond
    // collision with a numeric suffix (a base name still sorts before its `-N` variants,
    // so lexicographic order stays chronological). Serialized by the session mutex — no race.
    let base_name = store::snapshot_dir_name(now_ts);
    let mut dir_name = base_name.clone();
    let mut n: u32 = 1;
    while snapshots_dir.join(&dir_name).exists() {
        dir_name = format!("{base_name}-{n}");
        n = n.saturating_add(1);
    }

    let staging = store::staging_dir(snapshots_dir, &dir_name);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| io_ctx("create staging", &e))?;

    // 1. Live-safe VACUUM INTO the staging dir (dest must not exist — it is fresh).
    let vault_snap = staging.join(SNAPSHOT_VAULT_FILE);
    repo.vacuum_into(&vault_snap).await?;
    // 🔴 Null the copied-over Recovery Key slot (slice 5.7 ④) BEFORE hashing — a snapshot
    // carries no recovery credential; a live slot would re-arm recovery on the next revert.
    null_recovery_slot(&vault_snap).await?;
    let (_vault_size, vault_blake3) = archive::hash_file(&vault_snap)?;

    // 2. Deduplicate each blob into the content-addressed pool (Decision ⑪/⑫); build the
    //    object map the revert path reconstructs `blobs/{entry_id}.blob` from.
    let mut objects: Vec<SnapshotObject> = Vec::new();
    let mut blob_count: u64 = 0;
    let read_dir = match std::fs::read_dir(blobs_dir) {
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
        blob_files.sort(); // deterministic manifest order
        for src in blob_files {
            let entry_id = src
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    VaultError::Storage(StorageError::Io("blob file has a non-UTF-8 name".into()))
                })?
                .to_owned();
            let (blake3, size) = store::put_object(snapshots_dir, &src)?;
            objects.push(SnapshotObject {
                entry_id,
                blake3,
                size,
            });
            blob_count = blob_count.saturating_add(1);
        }
    }

    // 3. Build the manifest and write it LAST inside the staging dir.
    let manifest = SnapshotManifest {
        format_version: SNAPSHOT_FORMAT_VERSION,
        reason,
        vault_uuid: config.vault_uuid.clone(),
        schema_version: config.schema_version,
        created_at: created_at.clone(),
        commit_counter: config.commit_counter,
        verify_hash_prefix: verify_hash_prefix(&config.verify_hash),
        entry_count,
        blob_count,
        vault_blake3,
        objects,
    };
    store::write_manifest(&staging, &manifest)?;

    // 4. COMMIT POINT: atomic rename of the completed staging dir into place.
    let final_dir = snapshots_dir.join(&dir_name);
    std::fs::rename(&staging, &final_dir).map_err(|e| {
        std::fs::remove_dir_all(&staging).ok();
        io_ctx("commit snapshot dir", &e)
    })?;

    Ok(SnapshotReport {
        id: dir_name,
        created_at,
        reason,
        entry_count,
        blob_count,
    })
}

/// "Snapshot now" — capture a snapshot of the unlocked `session`'s vault.
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn create_snapshot(
    session: &VaultSession,
    reason: SnapshotReason,
) -> Result<SnapshotReport, VaultError> {
    let snapshots_dir = session.vault_id().snapshots_dir();
    let blobs_dir = session.blob.blob_dir().to_owned();
    // M2: ACTIVE entries only (`index.entries` includes trashed).
    let entry_count = u64::try_from(session.index.all_active().len()).unwrap_or(u64::MAX);

    let report = write_snapshot(
        &snapshots_dir,
        &blobs_dir,
        session.repo.as_ref(),
        reason,
        entry_count,
    )
    .await?;

    // ---- POST-COMMIT (L1): the snapshot is on disk. These side effects must not fail the
    //      operation — a best-effort failure is logged, never propagated. ----

    // `last_snapshot_at` — 🔴 NOT `last_backup_at` (Decision ⑧: a snapshot is not a backup).
    if let Err(e) = session.repo.touch_last_snapshot_at(now()).await {
        tracing::warn!(error = %e, "snapshot created but last_snapshot_at could not be recorded");
    }
    // One vault-level audit row, REUSING `BackupCreated` (no new AuditAction variant).
    if let Err(e) =
        super::create_entry::append_audit(session, AuditAction::BackupCreated, None).await
    {
        tracing::warn!(error = %e, "snapshot created but the audit row could not be written");
    }

    Ok(report)
}
