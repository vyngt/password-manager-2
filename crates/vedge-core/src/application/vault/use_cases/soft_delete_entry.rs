//! Soft delete — flip the trashed bit + stamp `trashed_at`.
//!
//! The ciphertext + DEK stay on disk so restore is a plain metadata flip;
//! retention cleanup (hard delete after N days) is `RunMaintenance`'s job.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn soft_delete_entry(
    session: &mut VaultSession,
    entry_id: &EntryId,
) -> Result<(), VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }
    let when = now();
    session.repo.soft_delete_entry(entry_id, when).await?;
    super::create_entry::append_audit(session, AuditAction::Deleted, Some(entry_id)).await?;

    if let Some(existing) = session.index.entries.get(entry_id).cloned() {
        let mut updated = existing;
        updated.is_trashed = true;
        updated.updated_at = when;
        session.index.update_entry(updated);
    }
    Ok(())
}
