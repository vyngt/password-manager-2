use zeroize::Zeroizing;

use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::errors::VaultError;

/// Hardware-gated custodian of a vault's KEK (Windows Hello / macOS Touch ID).
///
/// Biometric unlock does **not** change vault encryption and derives no key from the
/// fingerprint — it is an OS-hardware gate that releases a stored copy of the 32-byte
/// KEK (the key that unwraps every entry's DEK). The KEK is captured while the vault is
/// already unlocked ([`enroll`](Self::enroll)) and released only after a user-present
/// biometric check ([`retrieve`](Self::retrieve)). The master password remains the
/// source of truth and the required fallback; a stolen `.vdb` is still openable only
/// with password + Secret Key.
///
/// Synchronous — like [`KeychainProvider`](super::keychain::KeychainProvider), the
/// underlying OS calls are blocking and may show a modal prompt for several seconds.
/// Callers already invoke the sibling keychain port synchronously from async use cases
/// (see [`UnlockVault`](crate::application::vault::use_cases::UnlockVault)); the same
/// applies here.
///
/// Error mapping contract (infrastructure is responsible):
///
/// - No hardware / unsupported platform → [`VaultError::BiometricUnavailable`]
/// - No enrollment for this vault → [`VaultError::BiometricNotEnrolled`]
/// - User dismissed or failed the prompt → [`VaultError::BiometricCancelled`]
/// - Any other platform failure → [`VaultError::BiometricFailed`]
///
/// `VaultError::Display` never includes raw key bytes.
///
/// ## Keyed on `vault_uuid`, never the file path (slice 5.2.3)
///
/// Like [`KeychainProvider`](super::keychain::KeychainProvider) since 5.2.0, the credential
/// is addressed by the vault's intrinsic `vault_uuid` — a plaintext `vault_config` column —
/// so **moving or renaming a vault keeps its enrollment**. (Before 5.2.3 the key was
/// `SHA-256(path)`, so a convert or a move silently un-enrolled the user *and* stranded the
/// KEK-holding credential where nothing could ever reach it.) The one legacy path-keyed
/// credential a pre-5.2.3 vault may still hold is deleted by [`purge_legacy`](Self::purge_legacy)
/// during the layout migration.
pub trait BiometricAuthenticator: Send + Sync {
    /// Whether biometric hardware is present and usable on this device. Never prompts.
    fn is_available(&self) -> bool;

    /// Whether this vault currently has a KEK stored behind the biometric gate.
    /// Never prompts.
    fn is_enrolled(&self, vault_uuid: &str) -> Result<bool, VaultError>;

    /// Store (or replace) the vault's KEK behind the biometric gate. Requires a fresh
    /// user-present biometric check. Called only while the vault is unlocked, so the
    /// caller already holds the live KEK.
    fn enroll(&self, vault_uuid: &str, kek: &[u8; KEK_LEN]) -> Result<(), VaultError>;

    /// Release the stored KEK after a user-present biometric check (shows the OS
    /// prompt). Returns [`VaultError::BiometricNotEnrolled`] when nothing is stored.
    fn retrieve(&self, vault_uuid: &str) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError>;

    /// Remove the stored KEK for this vault. Idempotent — a missing enrollment is not
    /// an error.
    fn disable(&self, vault_uuid: &str) -> Result<(), VaultError>;

    /// Delete a **legacy** path-hashed credential (pre-5.2.0, keyed on `SHA-256(path)`) —
    /// the exact biometric mirror of [`KeychainProvider::migrate_secret_key`](
    /// super::keychain::KeychainProvider::migrate_secret_key)'s legacy half. Called once,
    /// best-effort, from the layout migration. Idempotent: `Ok(true)` when one existed,
    /// `Ok(false)` when none did.
    ///
    /// 🔴 This is the ONLY thing that will ever reach that credential. The vault's KEK is
    /// sealed behind it, and once the home is renamed the path it was keyed on is gone — so
    /// skipping this strands a KEK-holding credential in the TPM that no future version can
    /// delete. It does **not** re-enroll (that would need a Hello prompt mid-migration).
    fn purge_legacy(&self, legacy_vault_id: &VaultId) -> Result<bool, VaultError>;
}
