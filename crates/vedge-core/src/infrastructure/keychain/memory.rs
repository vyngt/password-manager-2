use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use zeroize::Zeroizing;

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

/// In-memory keychain for tests. Not intended for production — does nothing to protect
/// key material beyond zeroizing on `Drop`.
pub struct MemoryKeychainProvider {
    /// Secret Keys, keyed on `vault_uuid` (slice 5.2.0 — was keyed on `VaultId`).
    store: Mutex<HashMap<String, [u8; SECRET_KEY_LEN]>>,
    /// Rollback commit-counter baselines, keyed on `vault_uuid` (slice 5.2c). Plain
    /// integers, not secret — no zeroize needed.
    baselines: Mutex<HashMap<String, i64>>,
    /// Test knob: when set, `delete_secret_key` and `delete_commit_baseline` return an error
    /// — proves a failed credential delete never fails a vault delete (slice 5.2.4, L1).
    delete_fails: AtomicBool,
}

impl MemoryKeychainProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            baselines: Mutex::new(HashMap::new()),
            delete_fails: AtomicBool::new(false),
        }
    }

    /// Test knob: make every `delete_secret_key` / `delete_commit_baseline` fail, to prove a
    /// vault delete survives a keychain failure and reports it (slice 5.2.4).
    pub fn set_delete_fails(&self, fails: bool) {
        self.delete_fails.store(fails, Ordering::SeqCst);
    }
}

impl Default for MemoryKeychainProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MemoryKeychainProvider {
    fn drop(&mut self) {
        if let Ok(mut map) = self.store.lock() {
            for v in map.values_mut() {
                v.fill(0);
            }
        }
    }
}

impl KeychainProvider for MemoryKeychainProvider {
    fn read_secret_key(
        &self,
        vault_uuid: &str,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
        let guard = self
            .store
            .lock()
            .map_err(|_| VaultError::KeychainUnavailable)?;
        guard
            .get(vault_uuid)
            .copied()
            .map(Zeroizing::new)
            .ok_or(VaultError::KeychainEntryNotFound)
    }

    fn store_secret_key(
        &self,
        vault_uuid: &str,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError> {
        let Ok(mut guard) = self.store.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard.insert(vault_uuid.to_owned(), *key);
        Ok(())
    }

    fn delete_secret_key(&self, vault_uuid: &str) -> Result<(), VaultError> {
        if self.delete_fails.load(Ordering::SeqCst) {
            return Err(VaultError::KeychainAccessDenied);
        }
        let Ok(mut guard) = self.store.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard
            .remove(vault_uuid)
            .map_or(Err(VaultError::KeychainEntryNotFound), |mut v| {
                v.fill(0);
                Ok(())
            })
    }

    fn read_commit_baseline(&self, vault_uuid: &str) -> Result<Option<i64>, VaultError> {
        let guard = self
            .baselines
            .lock()
            .map_err(|_| VaultError::KeychainUnavailable)?;
        Ok(guard.get(vault_uuid).copied())
    }

    fn store_commit_baseline(&self, vault_uuid: &str, counter: i64) -> Result<(), VaultError> {
        let Ok(mut guard) = self.baselines.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard.insert(vault_uuid.to_owned(), counter);
        Ok(())
    }

    fn delete_commit_baseline(&self, vault_uuid: &str) -> Result<(), VaultError> {
        if self.delete_fails.load(Ordering::SeqCst) {
            return Err(VaultError::KeychainAccessDenied);
        }
        let Ok(mut guard) = self.baselines.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard.remove(vault_uuid); // idempotent — missing is fine
        Ok(())
    }
}
