//! Daily cleanup: trash retention, audit retention, orphan blob reaping.
//!
//! Safe to run at any time — reads the retention policy from
//! `vault_config` and acts on rows older than the cutoff.

use chrono::Duration;
use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::snapshot::manifest::{SnapshotReason, verify_hash_prefix};
use crate::infrastructure::snapshot::store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaintenanceReport {
    pub trashed_entries_deleted: u64,
    pub audit_events_deleted: u64,
    pub orphaned_blobs_deleted: u64,
    /// Whole snapshot dirs pruned by the opt-in retention policy (Decision ⑯).
    pub snapshots_pruned: u64,
    /// Content-addressed objects reclaimed by the mark-and-sweep GC (Decision ⑪).
    pub snapshot_objects_swept: u64,
}

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn run_maintenance(session: &VaultSession) -> Result<MaintenanceReport, VaultError> {
    let today = now();
    let trash_days = i64::from(session.config.trash_retention_days.max(0));
    let audit_days = i64::from(session.config.audit_retention_days.max(0));

    // ---- 1. Trashed entries past retention ---------------------------------
    let trash_cutoff = today
        .checked_sub_signed(Duration::days(trash_days))
        .ok_or_else(|| VaultError::MalformedPayload("trash retention cutoff overflow".into()))?;
    let trashed_entries_deleted = session
        .repo
        .hard_delete_trashed_before(trash_cutoff)
        .await?;

    // ---- 2. Audit rows past retention ---------------------------------------
    let audit_cutoff = today
        .checked_sub_signed(Duration::days(audit_days))
        .ok_or_else(|| VaultError::MalformedPayload("audit retention cutoff overflow".into()))?;
    let audit_events_deleted = session.repo.delete_audit_before(audit_cutoff).await?;

    // ---- 3. Orphaned blob files --------------------------------------------
    // A blob file is orphaned when no entry in the index refers to its ID.
    // Reads list → filter → delete each.
    let disk_ids: Vec<EntryId> = session.blob.list_blobs().await?;
    let mut orphaned_blobs_deleted: u64 = 0;
    for id in disk_ids {
        if !session.index.entries.contains_key(&id) {
            // Best-effort delete — a failure here just means the orphan
            // survives until next run.
            if session.blob.delete_blob(&id).await.is_ok() {
                orphaned_blobs_deleted = orphaned_blobs_deleted.saturating_add(1);
            }
        }
    }

    // ---- 4. Snapshot retention (Decision ⑯) + object GC (Decision ⑪) --------
    // Retention is OPT-IN: `backup_keep_count` unset ⇒ prune nothing (a password manager
    // must never silently delete a user's history). The object sweep runs UNCONDITIONALLY —
    // it reclaims objects orphaned by a user `delete_snapshot`, a crashed create, or a prune.
    let snapshots_dir = session.vault_id().snapshots_dir();
    let snapshots_pruned = prune_snapshots(session, &snapshots_dir);
    let snapshot_objects_swept = store::sweep_objects(&snapshots_dir).unwrap_or(0);

    Ok(MaintenanceReport {
        trashed_entries_deleted,
        audit_events_deleted,
        orphaned_blobs_deleted,
        snapshots_pruned,
        snapshot_objects_swept,
    })
}

/// Opt-in snapshot retention (Decision ⑯). Prunes whole snapshot dirs down toward
/// `backup_keep_count`, but with a HARD floor of 3 and four protections, ALL of which must
/// hold together or a user could lose their only good snapshot:
/// - never the newest `keep` (any reason);
/// - never a `pre-restore` snapshot (⑭ — that is somebody's undo button);
/// - never a stale-credential snapshot (⑬ — the only artifact the old kit can still open).
///
/// Best-effort per dir. `None` `backup_keep_count` ⇒ keep everything (returns 0).
fn prune_snapshots(session: &VaultSession, snapshots_dir: &std::path::Path) -> u64 {
    let Some(configured) = session.config.backup_keep_count else {
        return 0;
    };
    let keep = usize::try_from(configured.max(3)).unwrap_or(3);
    let live_prefix = verify_hash_prefix(&session.config.verify_hash);
    let all = store::list_snapshots(snapshots_dir).unwrap_or_default(); // newest-first

    let mut pruned: u64 = 0;
    for (i, snap) in all.iter().enumerate() {
        if i < keep {
            continue; // within the newest-`keep` window — always kept
        }
        if snap.manifest.reason == SnapshotReason::PreRestore {
            continue; // ⑯ never an undo point
        }
        if snap.manifest.verify_hash_prefix != live_prefix {
            continue; // ⑯ never a stale-credential snapshot (the old kit's only key to it)
        }
        if std::fs::remove_dir_all(&snap.dir).is_ok() {
            pruned = pruned.saturating_add(1);
        }
    }
    pruned
}
