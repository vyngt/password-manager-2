//! Daily cleanup: trash retention, audit retention, orphan blob reaping.
//!
//! Safe to run at any time — reads the retention policy from
//! `vault_config` and acts on rows older than the cutoff.

use chrono::Duration;
use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::errors::VaultError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_field_names)] // "_deleted" suffix carries meaning
pub struct MaintenanceReport {
    pub trashed_entries_deleted: u64,
    pub audit_events_deleted: u64,
    pub orphaned_blobs_deleted: u64,
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

    Ok(MaintenanceReport {
        trashed_entries_deleted,
        audit_events_deleted,
        orphaned_blobs_deleted,
    })
}
