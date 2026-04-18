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
    store: Mutex<HashMap<VaultId, [u8; SECRET_KEY_LEN]>>,
}

impl MemoryKeychainProvider {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
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
            for (_k, v) in map.iter_mut() {
                v.fill(0);
            }
        }
    }
}

impl KeychainProvider for MemoryKeychainProvider {
    fn read_secret_key(
        &self,
        vault_id: &VaultId,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
        let guard = self
            .store
            .lock()
            .map_err(|_| VaultError::KeychainUnavailable)?;
        guard
            .get(vault_id)
            .copied()
            .map(Zeroizing::new)
            .ok_or(VaultError::KeychainEntryNotFound)
    }

    fn store_secret_key(
        &self,
        vault_id: &VaultId,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError> {
        let Ok(mut guard) = self.store.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard.insert(vault_id.clone(), *key);
        Ok(())
    }

    fn delete_secret_key(&self, vault_id: &VaultId) -> Result<(), VaultError> {
        let Ok(mut guard) = self.store.lock() else {
            return Err(VaultError::KeychainUnavailable);
        };
        guard.remove(vault_id).map_or(
            Err(VaultError::KeychainEntryNotFound),
            |mut v| {
                v.fill(0);
                Ok(())
            },
        )
    }
}
