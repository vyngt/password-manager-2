use zeroize::Zeroizing;

use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

/// Access to the OS keychain for the per-vault Secret Key. Synchronous — platform
/// keychain APIs are blocking C calls; shell adapters wrap in `spawn_blocking` if
/// needed.
///
/// **Keyed on the intrinsic `vault_uuid`, never the file path** (slice 5.2.0). A vault
/// that is moved or renamed keeps its keychain entry — the whole point of 4.6a's
/// `vault_uuid`. The uuid is a plaintext `vault_config` column, readable with no KEK, so
/// a caller resolves it (from the loaded config or the session) before these calls.
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
        vault_uuid: &str,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError>;

    fn store_secret_key(
        &self,
        vault_uuid: &str,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError>;

    fn delete_secret_key(&self, vault_uuid: &str) -> Result<(), VaultError>;

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

    /// Migrate a **legacy** path-keyed Secret Key entry (`vault:{path}`, pre-5.2.0) to the
    /// uuid-keyed account (`secret:{uuid}`) — slice 5.2.0's layout migration.
    ///
    /// Order is **store → verify → delete-old**, so a crash between the store and the
    /// delete re-runs harmlessly and never loses the key. Idempotent: `Ok(true)` when a
    /// legacy entry was migrated, `Ok(false)` when none exists (already migrated, or the
    /// vault never had a keychain entry). `legacy_vault_id` is the OLD `.vdb` path the
    /// entry was keyed on.
    fn migrate_secret_key(
        &self,
        legacy_vault_id: &VaultId,
        vault_uuid: &str,
    ) -> Result<bool, VaultError>;
}
