use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::errors::VaultError;

/// Biometric authenticator for platforms without a native implementation.
///
/// Linux (password-only by design) and macOS until a future Touch ID follow-up.
/// Reports unavailable; enroll and retrieve fail with
/// [`VaultError::BiometricUnavailable`]; `disable` is a no-op so the settings
/// toggle can always turn off cleanly.
pub struct StubBiometricAuthenticator;

impl StubBiometricAuthenticator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for StubBiometricAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl BiometricAuthenticator for StubBiometricAuthenticator {
    fn is_available(&self) -> bool {
        false
    }

    fn is_enrolled(&self, _vault_id: &VaultId) -> Result<bool, VaultError> {
        Ok(false)
    }

    fn enroll(&self, _vault_id: &VaultId, _kek: &[u8; KEK_LEN]) -> Result<(), VaultError> {
        Err(VaultError::BiometricUnavailable)
    }

    fn retrieve(&self, _vault_id: &VaultId) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
        Err(VaultError::BiometricUnavailable)
    }

    fn disable(&self, _vault_id: &VaultId) -> Result<(), VaultError> {
        Ok(())
    }
}
