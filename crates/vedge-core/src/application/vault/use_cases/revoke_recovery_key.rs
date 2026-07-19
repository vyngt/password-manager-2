//! `revoke_recovery_key` — turn off the Recovery Key (slice 5.7).
//!
//! Deletes the slot, so future copies of this `.vdb` carry no recovery. 🔴 Honest scope: a
//! `.vdb` someone ALREADY copied keeps its old slot and can still recovery-unlock it — true
//! revocation of a leaked Recovery Key is a full re-key (5.8). The UI copy says so.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn revoke_recovery_key(session: &mut VaultSession) -> Result<(), VaultError> {
    session.repo.clear_recovery_slot().await?;
    session.config.recovery_slot = None;
    super::create_entry::append_audit(session, AuditAction::RecoveryKeyRevoked, None).await?;
    Ok(())
}
