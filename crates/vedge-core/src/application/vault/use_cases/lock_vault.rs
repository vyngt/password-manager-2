//! `lock_vault` — record the lock event and consume the session.
//!
//! Appending `AuditAction::Locked` must happen **before** the session
//! drops: once the session is gone the `repo` handle goes with it (held
//! inside `VaultSession`). We pull the `Arc` out, audit, then let the
//! session value drop.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::now;
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn lock_vault(session: VaultSession) -> Result<(), VaultError> {
    // Clone the repo Arc before consuming the session.
    let repo = std::sync::Arc::clone(&session.repo);

    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::Locked,
        occurred_at: now(),
        device_id: None,
    };
    let audit_result = repo.append_audit(&event).await;

    // Drop the session regardless of audit outcome — Drop zeroizes the KEK
    // and the VaultIndex. Propagate any audit error to the caller after.
    drop(session);
    audit_result
}
