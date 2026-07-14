use std::collections::HashMap;
use std::sync::Mutex;

use zeroize::Zeroizing;

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

/// In-memory keychain for tests. Not intended for production — does nothing to protect
/// key material beyond zeroizing on `Drop`.
pub struct MemoryKeychainProvider {
    /// Secret Keys, keyed on `vault_uuid` (slice 5.2.0 — was keyed on `VaultId`).
    store: Mutex<HashMap<String, [u8; SECRET_KEY_LEN]>>,
    /// LEGACY path-keyed Secret Keys, for migration-test support only (slice 5.2.0).
    /// Seeded via [`Self::seed_legacy_secret_key`]; drained by `migrate_secret_key`.
    legacy_store: Mutex<HashMap<VaultId, [u8; SECRET_KEY_LEN]>>,
    /// Rollback commit-counter baselines, keyed on `vault_uuid` (slice 5.2c). Plain
    /// integers, not secret — no zeroize needed.
    baselines: Mutex<HashMap<String, i64>>,
}

impl MemoryKeychainProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
            legacy_store: Mutex::new(HashMap::new()),
            baselines: Mutex::new(HashMap::new()),
        }
    }

    /// Seed a LEGACY path-keyed Secret Key, simulating a pre-5.2.0 vault whose keychain
    /// entry is still keyed on the file path — so a test can exercise `migrate_secret_key`.
    pub fn seed_legacy_secret_key(&self, vault_id: &VaultId, key: &[u8; SECRET_KEY_LEN]) {
        if let Ok(mut guard) = self.legacy_store.lock() {
            guard.insert(vault_id.clone(), *key);
        }
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
        if let Ok(mut map) = self.legacy_store.lock() {
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

    fn migrate_secret_key(
        &self,
        legacy_vault_id: &VaultId,
        vault_uuid: &str,
    ) -> Result<bool, VaultError> {
        let legacy = {
            let Ok(mut guard) = self.legacy_store.lock() else {
                return Err(VaultError::KeychainUnavailable);
            };
            guard.remove(legacy_vault_id)
        };
        match legacy {
            Some(key) => {
                self.store_secret_key(vault_uuid, &key)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
