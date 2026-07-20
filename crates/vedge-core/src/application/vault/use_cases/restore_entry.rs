//! Restore a soft-deleted entry — inverse of `soft_delete_entry`.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn restore_entry(
    session: &mut VaultSession,
    entry_id: &EntryId,
) -> Result<(), VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }
    let when = now();
    session.repo.restore_entry(entry_id, when).await?;
    super::create_entry::append_audit(session, AuditAction::Restored, Some(entry_id)).await?;

    if let Some(existing) = session.index.entries.get(entry_id).cloned() {
        let mut updated = existing;
        updated.is_trashed = false;
        updated.updated_at = when;
        session.index.update_entry(updated);
    }
    Ok(())
}
