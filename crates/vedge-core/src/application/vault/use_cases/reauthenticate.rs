//! `reauthenticate_master_password` — re-derive and authorize the current
//! master password against an unlocked vault's `verify_hash`.
//!
//! Extracted from [`enroll_biometric`](super::biometric::enroll_biometric),
//! which authorizes a Settings re-prompt exactly this way. The credential
//! lifecycle flows (change password, rotate Secret Key) reuse it to demand
//! proof of the current password **before** an O(n) rewrap — without a fourth
//! copy of the KDF chain. Returns the re-derived KEK so `enroll_biometric` can
//! store it; callers that only need the yes/no discard it.

use std::sync::Arc;

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::errors::VaultError;

/// Re-derive the KEK from a re-prompted `master_password` and authorize it
/// against the unlocked vault's stored `verify_hash`.
///
/// Resolves the *current* Secret Key from the OS keychain (the same source a
/// password unlock uses), runs the single Argon2 pass that yields both the
/// `verify_hash` and the KEK, and constant-time compares the hash. A wrong
/// password returns [`VaultError::WrongCredentials`] and derives nothing
/// further; a match returns the re-derived KEK.
///
/// The keychain read means a vault whose Secret Key is not on this device
/// (a biometric-only enrollment, or a partial recovery) cannot be
/// re-authorized here — the same precondition `enroll_biometric` already
/// carries.
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn reauthenticate_master_password(
    session: &VaultSession,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    master_password: Zeroizing<String>,
) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
    // 1. Resolve the Secret Key from the keychain (same source as unlock).
    let uuid = session
        .vault_uuid()
        .ok_or(VaultError::KeychainEntryNotFound)?;
    let secret_key = keychain.read_secret_key(uuid)?;

    // 2. One Argon2 pass → (KEK, verify_hash): authorizes the re-prompt AND
    //    yields the KEK. Runs on `spawn_blocking` so it never starves the
    //    async executor.
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
        // `kek_z` drops here, zeroizing the KEK we never return.
        return Err(VaultError::WrongCredentials);
    }

    Ok(kek_z)
}
