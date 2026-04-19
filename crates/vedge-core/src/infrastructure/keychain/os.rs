use keyring::{Entry, Error as KeyringError};
use zeroize::Zeroizing;

use crate::application::vault::ports::keychain::KeychainProvider;
use crate::domain::shared::VaultId;
use crate::domain::vault::crypto_constants::SECRET_KEY_LEN;
use crate::domain::vault::errors::VaultError;

const DEFAULT_SERVICE: &str = "vedge";

pub struct OsKeychainProvider {
    service: String,
}

impl OsKeychainProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            service: DEFAULT_SERVICE.to_owned(),
        }
    }

    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn account(vault_id: &VaultId) -> String {
        format!("vault:{}", vault_id.path().to_string_lossy())
    }

    fn entry(&self, vault_id: &VaultId) -> Result<Entry, VaultError> {
        Entry::new(&self.service, &Self::account(vault_id)).map_err(map_keyring_err)
    }
}

impl Default for OsKeychainProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::needless_pass_by_value)] // signature matches `map_err` combinator
fn map_keyring_err(err: KeyringError) -> VaultError {
    match err {
        KeyringError::PlatformFailure(_) | KeyringError::NoStorageAccess(_) => {
            VaultError::KeychainUnavailable
        }
        KeyringError::Ambiguous(_) => VaultError::KeychainAccessDenied,
        // NoEntry + corrupt-entry variants (BadEncoding / TooLong / Invalid) all surface
        // as "not found" — raw driver detail is intentionally hidden from callers.
        _ => VaultError::KeychainEntryNotFound,
    }
}

impl KeychainProvider for OsKeychainProvider {
    fn read_secret_key(
        &self,
        vault_id: &VaultId,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
        let entry = self.entry(vault_id)?;
        let mut bytes = entry.get_secret().map_err(map_keyring_err)?;
        let result = <[u8; SECRET_KEY_LEN]>::try_from(bytes.as_slice())
            .map_err(|_| VaultError::KeychainEntryNotFound);
        bytes.fill(0);
        result.map(Zeroizing::new)
    }

    fn store_secret_key(
        &self,
        vault_id: &VaultId,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError> {
        self.entry(vault_id)?
            .set_secret(key)
            .map_err(map_keyring_err)
    }

    fn delete_secret_key(&self, vault_id: &VaultId) -> Result<(), VaultError> {
        match self.entry(vault_id)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Err(VaultError::KeychainEntryNotFound),
            Err(e) => Err(map_keyring_err(e)),
        }
    }
}
