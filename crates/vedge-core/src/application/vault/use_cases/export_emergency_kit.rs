//! `export_emergency_kit` — build an [`EmergencyKitContent`] from an
//! unlocked session's Secret Key.
//!
//! ## Security flow
//!
//! 1. Caller has already unlocked the vault (holds a live `VaultSession`),
//!    so no additional password check is needed here.
//! 2. Read the raw Secret Key back from the OS keychain via the injected
//!    port. We deliberately do not store the Secret Key inside
//!    `VaultSession` — keeping it out of the session's lifetime limits the
//!    blast radius of a memory dump.
//! 3. Format the 16 bytes into the user-facing display string; zeroize the
//!    raw buffer immediately.
//! 4. Append an `AuditAction::Exported` event with `entry_id = None` to
//!    flag it as vault-level (as opposed to an entry document export).
//! 5. Return the content struct — rendering happens in the shell.

use std::sync::Arc;

use tracing::instrument;

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::now;
use crate::domain::vault::emergency_kit::EmergencyKitContent;
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::secret_key::format_secret_key;

#[derive(Debug)]
pub struct ExportEmergencyKitInput {
    /// Optional display name from `vault_registry`. Left to the caller
    /// because the recent-vaults table lives in `app.db`, not the per-vault
    /// DB that `VaultSession` wraps.
    pub vault_display_name: Option<String>,
}

#[instrument(skip_all)]
pub async fn export_emergency_kit(
    session: &VaultSession,
    keychain: Arc<dyn KeychainProvider>,
    input: ExportEmergencyKitInput,
) -> Result<EmergencyKitContent, VaultError> {
    // 1. Pull the Secret Key back from the keychain. `Zeroizing` on drop
    //    clears the raw bytes after `format_secret_key` has stringified
    //    them.
    let uuid = session
        .vault_uuid()
        .ok_or(VaultError::KeychainEntryNotFound)?;
    let secret_key = keychain.read_secret_key(uuid)?;

    // 2. Format once. The display string is the user's explicit output —
    //    not a `SecretString`, because the whole point is that it reaches
    //    a user-readable surface (PDF).
    let secret_key_display = format_secret_key(&secret_key);
    drop(secret_key); // explicit zeroization point

    // 3. Assemble content.
    let vault_name = input.vault_display_name.unwrap_or_default();
    let vault_path = session.vault_id().to_string();
    let kdf_params_summary = kdf_params_summary(&session.config.kdf_params);

    let content = EmergencyKitContent {
        vault_name,
        vault_path,
        generated_at: now(),
        secret_key_display,
        kdf_params_summary,
    };

    // 4. Audit. `Exported` + `entry_id = None` = vault-level export.
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::Exported,
        occurred_at: content.generated_at,
        device_id: None,
    };
    session.repo.append_audit(&event).await?;

    Ok(content)
}

fn kdf_params_summary(params: &crate::domain::vault::kdf_params::KdfParams) -> String {
    format!(
        "{alg} v{version} (m={m}, t={t}, p={p})",
        alg = params.alg,
        version = params.version,
        m = params.m,
        t = params.t,
        p = params.p,
    )
}
