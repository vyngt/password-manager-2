//! `enroll_biometric` — hand the current vault's KEK to the platform biometric gate.
//!
//! Enrollment happens while the vault is already unlocked (from Settings), so the KEK is
//! reachable. Rather than reading it straight out of `session.kek`, we re-derive it from
//! the re-prompted master password in a single Argon2 pass that **also** produces the
//! `verify_hash` — one operation both authorizes the "confirm your master password"
//! gate and yields the exact KEK a password unlock would produce. Only then is the KEK
//! stored behind the biometric gate (which shows the OS prompt).
//!
//! Disabling and the availability/enrolled queries are trivial pass-throughs to the
//! [`BiometricAuthenticator`] port, so the shell calls those directly.

use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::vault::errors::VaultError;

/// Enroll the unlocked vault for biometric unlock.
///
/// Verifies the re-prompted `master_password` against the vault's `verify_hash`, then
/// stores the derived KEK behind the biometric gate. A wrong password returns
/// [`VaultError::WrongCredentials`] and stores nothing; an unavailable authenticator
/// returns [`VaultError::BiometricUnavailable`].
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn enroll_biometric(
    session: &VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    master_password: Zeroizing<String>,
) -> Result<(), VaultError> {
    if !biometric.is_available() {
        return Err(VaultError::BiometricUnavailable);
    }

    // Keyed on `vault_uuid` (slice 5.2.3) so a later move/rename keeps the enrollment.
    let uuid = session
        .vault_uuid()
        .ok_or(VaultError::KeychainEntryNotFound)?;

    // Authorize the re-prompt AND obtain the KEK to store — one Argon2 pass,
    // shared with the credential-lifecycle flows. A wrong password returns
    // `WrongCredentials` and stores nothing.
    let kek_z = super::reauthenticate::reauthenticate_master_password(
        session,
        kdf,
        keychain,
        master_password,
    )
    .await?;

    // Store the KEK behind the biometric gate (shows the OS prompt). `kek_z`
    // zeroizes on drop immediately after.
    biometric.enroll(uuid, &kek_z)?;
    Ok(())
}
