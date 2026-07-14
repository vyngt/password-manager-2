//! Revert a vault IN PLACE to one of its local snapshots (slice 5.2.1) — the destructive
//! half of Decision ⑦'s TIME axis, and undoable.
//!
//! A **file** operation, like `restore_vault`: no session, no KEK (you must be able to revert
//! a vault you cannot open — the H0 disaster path). It reuses 5.2b's journaled crash-safe
//! swap, but with two differences from a `.vbk` restore:
//!
//! 1. 🟢 It **auto-snapshots the current vault first** (Decision ⑭), tagged `pre-restore`, so
//!    a wrong revert is itself revertible. Skipped only when the target is unreadable — there
//!    is nothing to preserve.
//! 2. 🔴 It swaps with `journal::commit_preserving(&[SNAPSHOTS_DIR])` so the home's
//!    `snapshots/` store SURVIVES the swap (Decision ⑮). The store lives inside the home a
//!    revert replaces — a plain whole-home swap would delete the safety net it is made of.
//!
//! The snapshot is self-verifying (each object's BLAKE3 == its own filename), so a corrupt
//! target disarms no guard: the authority is the snapshot's manifest, and the store is ours.

use std::path::PathBuf;

use tracing::{instrument, warn};

use crate::application::vault::ports::factories::VaultRepositoryFactory;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::shared::{
    BLOBS_DIR, SNAPSHOTS_DIR, StorageError, VAULT_FILE, format_rfc3339_millis, now,
};
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::target::read_target_identity;
use crate::infrastructure::backup::{archive, journal};
use crate::infrastructure::snapshot::manifest::{SNAPSHOT_FORMAT_VERSION, SnapshotReason};
use crate::infrastructure::snapshot::store;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use super::create_snapshot::write_snapshot;

/// Which snapshot to revert this vault to.
#[derive(Debug, Clone)]
pub struct RevertToSnapshotInput {
    /// The vault HOME directory (`<name>.vedge/`).
    pub vault: PathBuf,
    /// The snapshot's directory name (its id). UNTRUSTED — resolved + containment-checked.
    pub snapshot_id: String,
    /// The user's explicit yes to the rollback (reverting is a rollback by design). When the
    /// live vault is AHEAD of the snapshot, revert refuses unless this is `true` — ⑭ makes it
    /// undoable, so this is the expected case, not a hazard.
    pub confirm_rollback: bool,
}

/// Non-invertible facts surfaced to the UI.
#[derive(Debug, Clone)]
pub struct RevertReport {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub reverted_at: String,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Re-baseline the keychain rollback mirror to the REVERTED vault's own counter (best-effort;
/// the swap already committed, so a keychain failure must not fail the revert — L1). Mirrors
/// `restore_vault::rebaseline_rollback_mirror`.
async fn rebaseline_rollback_mirror(repo: &dyn VaultRepository, keychain: &dyn KeychainProvider) {
    let cfg = match repo.load_config().await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(error = %e, "revert committed but re-reading config for re-baseline failed");
            return;
        }
    };
    let Some(uuid) = cfg.vault_uuid.as_deref() else {
        return;
    };
    if let Err(e) = keychain.store_commit_baseline(uuid, cfg.commit_counter) {
        warn!(error = %e, "revert committed but the rollback baseline could not be re-based");
    }
}

/// Decision ⑭: capture a `pre-restore` snapshot of the CURRENT vault before the swap so the
/// revert is undoable. Best-effort — a failure (a corrupt/unreadable target) is logged and
/// skipped, never fatal.
///
/// 🔴 Opens a `VaultDbConnection` DIRECTLY and `close()`s it before returning, rather than a
/// port `Arc<dyn VaultRepository>` — the connection MUST be released before the swap renames
/// the home, and Windows will not rename a directory holding an open `.vdb` handle. sqlx's
/// `Drop` close is async, so an explicit close is the only guarantee (M1's retry is the
/// backstop, not the mechanism).
async fn auto_snapshot_pre_restore(
    home: &std::path::Path,
    snapshots_dir: &std::path::Path,
    entry_count: u64,
) {
    let db = match VaultDbConnection::open(&home.join(VAULT_FILE)).await {
        Ok(db) => db,
        Err(e) => {
            warn!(error = %e, "target unreadable — skipping the pre-restore auto-snapshot (⑭)");
            return;
        }
    };
    let repo = SqliteVaultRepository::new(db.handle());
    let blobs_dir = home.join(BLOBS_DIR);
    if let Err(e) = write_snapshot(
        snapshots_dir,
        &blobs_dir,
        &repo,
        SnapshotReason::PreRestore,
        entry_count,
    )
    .await
    {
        warn!(error = %e, "pre-restore auto-snapshot failed — proceeding with the revert (⑭)");
    }
    drop(repo); // release the repo's handle clone so `close()` can actually close the pool
    if let Err(e) = db.close().await {
        warn!(error = %e, "closing the pre-restore connection failed before the swap");
    }
}

#[instrument(skip_all, fields(vault = %input.vault.display(), snapshot = %input.snapshot_id))]
pub async fn revert_to_snapshot(
    repo_factory: &dyn VaultRepositoryFactory,
    keychain: &dyn KeychainProvider,
    input: RevertToSnapshotInput,
) -> Result<RevertReport, VaultError> {
    let home = input.vault;
    let snapshots_dir = home.join(SNAPSHOTS_DIR);

    // 0. Reconcile any prior interrupted restore of this home before a fresh swap.
    journal::recover_if_pending(&home)?;

    // 1. Resolve + verify the snapshot. Self-verifying: each object's BLAKE3 == its filename,
    //    and the snapshot's `vault.vdb` matches the manifest — no trust in the (maybe corrupt)
    //    live target is required.
    let snap_dir = store::resolve_snapshot_dir(&snapshots_dir, &input.snapshot_id)
        .map_err(|_| VaultError::SnapshotNotFound(input.snapshot_id.clone()))?;
    let manifest = store::read_manifest(&snap_dir)
        .map_err(|e| VaultError::SnapshotManifestUnreadable(e.to_string()))?;
    if manifest.format_version != SNAPSHOT_FORMAT_VERSION {
        return Err(VaultError::SnapshotUnsupportedFormat(
            manifest.format_version,
        ));
    }
    let snap_vault = snap_dir.join(VAULT_FILE);
    let (_vsize, snap_vault_hash) = archive::hash_file(&snap_vault)
        .map_err(|_| VaultError::SnapshotCorrupt("snapshot vault.vdb is unreadable".into()))?;
    if snap_vault_hash != manifest.vault_blake3 {
        return Err(VaultError::SnapshotCorrupt(
            "snapshot vault.vdb failed verification".into(),
        ));
    }
    for obj in &manifest.objects {
        let obj_path = store::object_path(&snapshots_dir, &obj.blake3);
        let (_s, hash) = archive::hash_file(&obj_path).map_err(|_| {
            VaultError::SnapshotCorrupt(format!("missing object b3-{}", obj.blake3))
        })?;
        if hash != obj.blake3 {
            return Err(VaultError::SnapshotCorrupt(format!(
                "object b3-{} failed verification",
                obj.blake3
            )));
        }
    }

    // 2. Rollback gate + ⑭ auto-snapshot, both gated on the target being READABLE. A `None`
    //    identity means a corrupt/missing target: nothing to roll back over, nothing to
    //    preserve — proceed straight to the revert (the H0 disaster path).
    if let Some(id) = read_target_identity(&home).await {
        if let Some(target_ctr) = id.commit_counter {
            if target_ctr > manifest.commit_counter && !input.confirm_rollback {
                return Err(VaultError::RollbackNotConfirmed);
            }
        }
        auto_snapshot_pre_restore(&home, &snapshots_dir, id.entry_count).await;
    }

    // 3. Stage the chosen snapshot as the new home: its `vault.vdb` + blobs reconstructed from
    //    the object pool + an EMPTY `snapshots/` (the preserving swap fills it from `.old`).
    let staging = journal::staging_path(&home);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| io_ctx("create staging", &e))?;

    let staged_vault = journal::staged_vault(&staging);
    std::fs::copy(&snap_vault, &staged_vault).map_err(|e| io_ctx("stage snapshot vault", &e))?;
    journal::fsync_file(&staged_vault)?;

    let staged_blobs = journal::staged_blobs(&staging);
    std::fs::create_dir_all(&staged_blobs).map_err(|e| io_ctx("create staged blobs", &e))?;
    std::fs::create_dir_all(staging.join(SNAPSHOTS_DIR))
        .map_err(|e| io_ctx("create staged snapshots", &e))?;

    let mut blob_count: u64 = 0;
    for obj in &manifest.objects {
        let blob_name = format!("{}.blob", obj.entry_id);
        if !journal::is_safe_blob_name(&blob_name) {
            std::fs::remove_dir_all(&staging).ok();
            return Err(VaultError::SnapshotCorrupt(format!(
                "snapshot names an unsafe blob: {}",
                obj.entry_id
            )));
        }
        let src = store::object_path(&snapshots_dir, &obj.blake3);
        let dest = staged_blobs.join(&blob_name);
        std::fs::copy(&src, &dest).map_err(|e| io_ctx("reconstruct blob from object", &e))?;
        journal::fsync_file(&dest)?;
        blob_count = blob_count.saturating_add(1);
    }
    journal::fsync_dir(&staging);

    // Re-hash the staged vault — anchors the journal (what recovery re-verifies).
    let (_s, staged_hash) = archive::hash_file(&staged_vault)?;

    // 4. COMMIT: the write-ahead swap, PRESERVING `snapshots/` (§A / Decision ⑮).
    let started_at = format_rfc3339_millis(now());
    journal::commit_preserving(&home, &staging, &staged_hash, &started_at, &[SNAPSHOTS_DIR])?;

    // 5. POST (L1): reopen (fatal — signals a bad revert) + one best-effort audit row + the
    //    best-effort rollback-mirror re-baseline. Nothing here may fail the revert.
    let repo = repo_factory.open(&home).await?;
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::BackupRestored,
        occurred_at: now(),
        device_id: None,
    };
    if let Err(e) = repo.append_audit(&event).await {
        warn!(error = %e, "revert committed but the audit row could not be written");
    }
    rebaseline_rollback_mirror(repo.as_ref(), keychain).await;

    Ok(RevertReport {
        vault_uuid: manifest.vault_uuid.clone(),
        entry_count: manifest.entry_count,
        blob_count,
        reverted_at: started_at,
    })
}
