//! Hard delete — remove the ciphertext forever.
//!
//! Order of operations (per spec):
//! 1. Refuse if the entry is a Folder with children (`FolderNotEmpty`).
//! 2. **Append the audit event first** — the entry ID must still be valid
//!    when the audit row lands. If the DB delete fails later, the audit
//!    row stays as a dangling reference (acceptable; spec: `audit_log` retains
//!    IDs past entry deletion).
//! 3. Delete the entry row via the repo.
//! 4. For `Document` entries, best-effort delete the blob file. Failure
//!    here only leaves an orphan on disk — `RunMaintenance` cleans orphans
//!    on the next daily run, so we swallow the error rather than fail the
//!    hard-delete after the DB row is already gone.
//! 5. Remove from the `VaultIndex`.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::EntryId;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryType;

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn hard_delete_entry(
    session: &mut VaultSession,
    entry_id: &EntryId,
) -> Result<(), VaultError> {
    let existing = session
        .index
        .entries
        .get(entry_id)
        .ok_or_else(|| VaultError::EntryNotFound(entry_id.clone()))?
        .clone();

    if existing.entry_type == EntryType::Folder {
        let has_children = session
            .index
            .entries
            .values()
            .any(|e| e.folder_id.as_ref() == Some(entry_id));
        if has_children {
            return Err(VaultError::FolderNotEmpty);
        }
    }

    // Audit while the entry still exists.
    super::create_entry::append_audit(session, AuditAction::PermanentlyDeleted, Some(entry_id))
        .await?;

    session.repo.hard_delete_entry(entry_id).await?;

    if existing.entry_type == EntryType::Document {
        // Best-effort — failure here means the blob file outlives its
        // entry row; `RunMaintenance` will reap it.
        if let Err(e) = session.blob.delete_blob(entry_id).await {
            tracing::warn!(
                entry_id = %entry_id,
                error = ?std::mem::discriminant(&e),
                "blob delete failed; orphan will be reaped by run_maintenance"
            );
        }
    }

    session.index.remove_entry(entry_id);
    Ok(())
}
