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
use crate::domain::vault::crypto_constants::KEK_LEN;
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

    // 1. Resolve the Secret Key from the keychain (same source as unlock).
    let uuid = session
        .vault_uuid()
        .ok_or(VaultError::KeychainEntryNotFound)?;
    let secret_key = keychain.read_secret_key(uuid)?;

    // 2. One Argon2 pass → (KEK, verify_hash): authorizes the re-prompt AND yields the
    //    KEK to store. Runs on `spawn_blocking` so it never starves the async executor.
    let vault_salt = session.config.vault_salt;
    let kdf_params = session.config.kdf_params.clone();
    let kdf_job = Arc::clone(&kdf);

    let (kek_z, verify_hash) = tokio::task::spawn_blocking(
        move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; 32]), VaultError> {
            let input_bytes = kdf_job.preprocess_2skd(master_password.as_bytes(), &secret_key)?;
            let master_key = kdf_job.derive_master_key(&input_bytes, &vault_salt, &kdf_params)?;
            let verify = kdf_job.derive_verify_hash(&master_key)?;
            let kek = kdf_job.derive_kek(&master_key)?;
            Ok((kek, verify))
        },
    )
    .await
    .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))??;

    // 3. Authorize: the re-prompted password must match the vault's verify hash.
    if !session
        .crypto
        .verify_hash_matches(&verify_hash, &session.config.verify_hash)
    {
        // `kek_z` drops here, zeroizing the KEK we never store.
        return Err(VaultError::WrongCredentials);
    }

    // 4. Store the KEK behind the biometric gate (shows the OS prompt). `kek_z`
    //    zeroizes on drop immediately after. Keyed on `vault_uuid` (slice 5.2.3), the same
    //    `uuid` the keychain read used above — so a later move/rename keeps the enrollment.
    biometric.enroll(uuid, &kek_z)?;
    Ok(())
}
