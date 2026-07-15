use std::collections::HashMap;
use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::errors::VaultError;

/// In-memory biometric authenticator for tests.
///
/// Reports available by default and never prompts, so it exercises the full enroll →
/// retrieve → disable lifecycle headlessly. Keyed on `vault_uuid` (slice 5.2.3 — was keyed
/// on `VaultId`), so a moved/renamed vault keeps its enrollment, mirroring the real Windows
/// impl. Zeroizes stored KEKs on `Drop`. Not for production.
pub struct MemoryBiometricAuthenticator {
    available: bool,
    /// Enrollments keyed on `vault_uuid` (slice 5.2.3).
    store: Mutex<HashMap<String, [u8; KEK_LEN]>>,
}

impl MemoryBiometricAuthenticator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            available: true,
            store: Mutex::new(HashMap::new()),
        }
    }

    /// Simulate a device with no biometric hardware (`is_available() == false`).
    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            available: false,
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryBiometricAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MemoryBiometricAuthenticator {
    fn drop(&mut self) {
        if let Ok(mut map) = self.store.lock() {
            for v in map.values_mut() {
                v.fill(0);
            }
        }
    }
}

impl BiometricAuthenticator for MemoryBiometricAuthenticator {
    fn is_available(&self) -> bool {
        self.available
    }

    fn is_enrolled(&self, vault_uuid: &str) -> Result<bool, VaultError> {
        let guard = self
            .store
            .lock()
            .map_err(|_| VaultError::BiometricFailed("memory store poisoned".to_owned()))?;
        Ok(guard.contains_key(vault_uuid))
    }

    fn enroll(&self, vault_uuid: &str, kek: &[u8; KEK_LEN]) -> Result<(), VaultError> {
        if !self.available {
            return Err(VaultError::BiometricUnavailable);
        }
        let mut guard = self
            .store
            .lock()
            .map_err(|_| VaultError::BiometricFailed("memory store poisoned".to_owned()))?;
        guard.insert(vault_uuid.to_owned(), *kek);
        drop(guard);
        Ok(())
    }

    fn retrieve(&self, vault_uuid: &str) -> Result<Zeroizing<[u8; KEK_LEN]>, VaultError> {
        if !self.available {
            return Err(VaultError::BiometricUnavailable);
        }
        let guard = self
            .store
            .lock()
            .map_err(|_| VaultError::BiometricFailed("memory store poisoned".to_owned()))?;
        guard
            .get(vault_uuid)
            .copied()
            .map(Zeroizing::new)
            .ok_or(VaultError::BiometricNotEnrolled)
    }

    fn disable(&self, vault_uuid: &str) -> Result<(), VaultError> {
        let mut guard = self
            .store
            .lock()
            .map_err(|_| VaultError::BiometricFailed("memory store poisoned".to_owned()))?;
        if let Some(mut v) = guard.remove(vault_uuid) {
            v.fill(0);
        }
        drop(guard);
        Ok(())
    }
}
