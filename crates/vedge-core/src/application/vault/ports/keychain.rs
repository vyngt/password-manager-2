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

    /// Read the mirrored rollback commit-counter baseline for a vault (slice 5.2c).
    ///
    /// Keyed on the intrinsic `vault_uuid` (never the path) so the baseline survives a
    /// vault move. **Deliberately unlike [`Self::read_secret_key`]: a missing entry is
    /// `Ok(None)`, not an error.** A brand-new device has never written a baseline, and
    /// that case must neither fail unlock nor raise a rollback warning — only a daemon /
    /// access failure is an `Err`.
    fn read_commit_baseline(&self, vault_uuid: &str) -> Result<Option<i64>, VaultError>;

    /// Store (or overwrite) the rollback commit-counter baseline for a vault (5.2c).
    fn store_commit_baseline(&self, vault_uuid: &str, counter: i64) -> Result<(), VaultError>;
}
