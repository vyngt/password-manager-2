//! `change_password_after_recovery` — the forced new master password after a recovery unlock
//! (slice 5.7 ③⑤).
//!
//! A recovery unlock proved possession of `Recovery Key + Secret Key`, so this path does NOT
//! re-authenticate the OLD password (the user forgot it — demanding it is a contradiction).
//! To keep "no-reauth password change" from becoming a walk-up-attacker escalation on ANY
//! unlocked vault, it is gated on a **one-shot session capability**: only a session freshly
//! opened by `unlock_with_recovery_key` carries `pending_recovery_reset`. Consuming it here
//! makes the no-reauth change reachable exactly once, on exactly that session.
//!
//! The stashed Secret Key is passed to `change_password` explicitly (as the SAME key), which
//! keeps it unchanged AND guarantees it is re-stored — so the change works even if the
//! keychain re-store at recovery time failed on a fresh device. Via ③, `change_password` also
//! nulls the now-defunct recovery slot, so recovery ends off and the user re-enrols.

use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::vault::errors::VaultError;

use super::change_password::{ChangePasswordInput, change_password};

#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn change_password_after_recovery(
    session: &mut VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    new_password: Zeroizing<String>,
) -> Result<(), VaultError> {
    // The one-shot gate: absent ⇒ this session was not opened by recovery, so a no-reauth
    // password change is refused.
    let secret_key = session
        .pending_recovery_reset
        .take()
        .ok_or(VaultError::NoRecoveryResetPending)?;

    change_password(
        session,
        kdf,
        keychain,
        biometric,
        ChangePasswordInput {
            new_password,
            // The SAME Secret Key, supplied explicitly: keeps it unchanged, re-stores it, and
            // sidesteps the keychain read the normal path would do.
            new_secret_key: Some(secret_key),
        },
    )
    .await
}
