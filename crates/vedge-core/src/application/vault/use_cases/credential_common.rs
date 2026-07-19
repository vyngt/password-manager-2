//! Shared credential-rotation primitives used by both `change_password` (5.6, the fast
//! O(n) rewrap) and `rekey_vault` (5.8, the full re-encrypt-everything superset).
//!
//! Both operations mint a *new KEK* from a password + Secret Key and, on success, re-store
//! the (possibly rotated) Secret Key and the biometric-gated KEK. Extracting the two shared
//! pieces here keeps re-key a genuine superset of change-password rather than a fork that can
//! drift from it (spec 5.8 ④).

use std::sync::Arc;

use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::kdf::KeyDerivationProvider;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::vault::crypto_constants::{
    KEK_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN, VERIFY_HASH_LEN,
};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::kdf_params::KdfParams;

/// Derive a fresh `(KEK, verify_hash)` from a master password + Secret Key on a blocking
/// thread — the 2SKD → Argon2id → HKDF chain.
///
/// Argon2id is CPU-bound (~500 ms), so it runs inside `spawn_blocking` to keep the async
/// executor free. `vault_salt` and `kdf_params` are the vault's own — a KEK change reuses
/// them, so only the *input* (the new credential) changes.
pub(crate) async fn derive_kek_and_verify(
    kdf: Arc<dyn KeyDerivationProvider>,
    password: Zeroizing<String>,
    secret_key: Zeroizing<[u8; SECRET_KEY_LEN]>,
    vault_salt: [u8; VAULT_SALT_LEN],
    kdf_params: KdfParams,
) -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; VERIFY_HASH_LEN]), VaultError> {
    tokio::task::spawn_blocking(
        move || -> Result<(Zeroizing<[u8; KEK_LEN]>, [u8; VERIFY_HASH_LEN]), VaultError> {
            let input_bytes = kdf.preprocess_2skd(password.as_bytes(), &secret_key)?;
            let master_key = kdf.derive_master_key(&input_bytes, &vault_salt, &kdf_params)?;
            let verify = kdf.derive_verify_hash(&master_key)?;
            let kek = kdf.derive_kek(&master_key)?;
            Ok((kek, verify))
        },
    )
    .await
    .map_err(|e| VaultError::KeyDerivationFailed(format!("spawn_blocking join: {e}")))?
}

/// After a KEK change has committed, re-store the credentials the OS holds outside the vault:
/// the Secret Key (only if it was rotated) and the biometric-gated KEK (only if enrolled).
///
/// Both are keyed by the intrinsic `vault_uuid` (slice 5.2.3), never the path. Called only
/// **after** the atomic commit so a rollback never leaves the keychain / gate holding key
/// material the on-disk vault no longer matches. The stored biometric KEK is otherwise stale
/// (it can't unwrap the freshly-rewrapped DEKs), so re-storing the new KEK is what keeps
/// biometric unlock working.
pub(crate) fn restore_keychain_and_biometric(
    keychain: &dyn KeychainProvider,
    biometric: &dyn BiometricAuthenticator,
    vault_uuid: &str,
    new_kek: &[u8; KEK_LEN],
    new_secret_key: Option<&[u8; SECRET_KEY_LEN]>,
) -> Result<(), VaultError> {
    if let Some(sk) = new_secret_key {
        keychain.store_secret_key(vault_uuid, sk)?;
    }
    if biometric.is_enrolled(vault_uuid)? {
        biometric.enroll(vault_uuid, new_kek)?;
    }
    Ok(())
}
