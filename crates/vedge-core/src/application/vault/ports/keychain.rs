use zeroize::Zeroizing;

use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

/// Access to the OS keychain for the per-vault Secret Key. Synchronous — platform
/// keychain APIs are blocking C calls; shell adapters wrap in `spawn_blocking` if
/// needed.
///
/// Error mapping contract (infrastructure is responsible):
///
/// - Missing entry → [`VaultError::KeychainEntryNotFound`]
/// - Daemon/storage unavailable → [`VaultError::KeychainUnavailable`]
/// - Auth / permission failure → [`VaultError::KeychainAccessDenied`]
///
/// `VaultError::Display` never includes raw key bytes.
pub trait KeychainProvider: Send + Sync {
    fn read_secret_key(
        &self,
        vault_id: &VaultId,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError>;

    fn store_secret_key(
        &self,
        vault_id: &VaultId,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError>;

    fn delete_secret_key(&self, vault_id: &VaultId) -> Result<(), VaultError>;
}
