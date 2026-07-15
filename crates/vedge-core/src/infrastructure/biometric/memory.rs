use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use zeroize::Zeroizing;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::domain::shared::VaultId;
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
    /// LEGACY path-keyed enrollments, for migration-test support only (slice 5.2.3).
    /// Seeded via [`Self::seed_legacy_credential`]; drained by `purge_legacy`. Mirrors the
    /// keychain double's `legacy_store`.
    legacy_store: Mutex<HashMap<VaultId, [u8; KEK_LEN]>>,
    /// Test knob: when set, `purge_legacy` returns an error — proves a failed purge never
    /// fails the layout migration (slice 5.2.3, L1).
    purge_fails: AtomicBool,
}

impl MemoryBiometricAuthenticator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            available: true,
            store: Mutex::new(HashMap::new()),
            legacy_store: Mutex::new(HashMap::new()),
            purge_fails: AtomicBool::new(false),
        }
    }

    /// Simulate a device with no biometric hardware (`is_available() == false`).
    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            available: false,
            store: Mutex::new(HashMap::new()),
            legacy_store: Mutex::new(HashMap::new()),
            purge_fails: AtomicBool::new(false),
        }
    }

    /// Seed a LEGACY path-keyed enrollment, simulating a pre-5.2.3 vault whose biometric
    /// credential is still keyed on the file path — so a test can exercise `purge_legacy`.
    pub fn seed_legacy_credential(&self, vault_id: &VaultId, kek: &[u8; KEK_LEN]) {
        if let Ok(mut guard) = self.legacy_store.lock() {
            guard.insert(vault_id.clone(), *kek);
        }
    }

    /// Test knob: make the next (and every) `purge_legacy` fail, to prove the migration
    /// survives it (slice 5.2.3).
    pub fn set_purge_fails(&self, fails: bool) {
        self.purge_fails.store(fails, Ordering::SeqCst);
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
        if let Ok(mut map) = self.legacy_store.lock() {
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

    fn purge_legacy(&self, legacy_vault_id: &VaultId) -> Result<bool, VaultError> {
        if self.purge_fails.load(Ordering::SeqCst) {
            return Err(VaultError::BiometricFailed(
                "purge failed (test knob)".to_owned(),
            ));
        }
        let mut guard = self
            .legacy_store
            .lock()
            .map_err(|_| VaultError::BiometricFailed("memory store poisoned".to_owned()))?;
        Ok(guard.remove(legacy_vault_id).is_some_and(|mut v| {
            v.fill(0);
            true
        }))
    }
}
