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

    /// The Secret-Key account, keyed on the intrinsic `vault_uuid` (slice 5.2.0). Was
    /// `vault:{path}` — a path-keyed entry silently lost its key on any move/rename.
    fn account(vault_uuid: &str) -> String {
        format!("secret:{vault_uuid}")
    }

    fn entry(&self, vault_uuid: &str) -> Result<Entry, VaultError> {
        Entry::new(&self.service, &Self::account(vault_uuid)).map_err(map_keyring_err)
    }

    /// A SECOND account namespace under the same `"vedge"` service, keyed on the
    /// intrinsic `vault_uuid` (not the path) so the rollback baseline survives a vault
    /// move — which is exactly what 4.6a's `vault_uuid` exists for (slice 5.2c).
    fn counter_account(vault_uuid: &str) -> String {
        format!("counter:{vault_uuid}")
    }

    fn counter_entry(&self, vault_uuid: &str) -> Result<Entry, VaultError> {
        Entry::new(&self.service, &Self::counter_account(vault_uuid)).map_err(map_keyring_err)
    }

    /// The LEGACY (pre-5.2.0) path-keyed Secret-Key account — read only by
    /// `migrate_secret_key` to move the entry onto `secret:{uuid}`.
    fn legacy_account(vault_id: &VaultId) -> String {
        format!("vault:{}", vault_id.path().to_string_lossy())
    }

    fn legacy_entry(&self, vault_id: &VaultId) -> Result<Entry, VaultError> {
        Entry::new(&self.service, &Self::legacy_account(vault_id)).map_err(map_keyring_err)
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
        vault_uuid: &str,
    ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
        let entry = self.entry(vault_uuid)?;
        let mut bytes = entry.get_secret().map_err(map_keyring_err)?;
        let result = <[u8; SECRET_KEY_LEN]>::try_from(bytes.as_slice())
            .map_err(|_| VaultError::KeychainEntryNotFound);
        bytes.fill(0);
        result.map(Zeroizing::new)
    }

    fn store_secret_key(
        &self,
        vault_uuid: &str,
        key: &[u8; SECRET_KEY_LEN],
    ) -> Result<(), VaultError> {
        self.entry(vault_uuid)?
            .set_secret(key)
            .map_err(map_keyring_err)
    }

    fn delete_secret_key(&self, vault_uuid: &str) -> Result<(), VaultError> {
        match self.entry(vault_uuid)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Err(VaultError::KeychainEntryNotFound),
            Err(e) => Err(map_keyring_err(e)),
        }
    }

    fn read_commit_baseline(&self, vault_uuid: &str) -> Result<Option<i64>, VaultError> {
        // A missing entry is `Ok(None)` (fresh device), NOT an error — see the trait
        // doc. Only a daemon/access failure escalates. A corrupt (wrong-length) value
        // reads as "no baseline" rather than a hard error: a bad mirror must never
        // brick unlock.
        match self.counter_entry(vault_uuid)?.get_secret() {
            Ok(bytes) => <[u8; 8]>::try_from(bytes.as_slice())
                .map_or(Ok(None), |arr| Ok(Some(i64::from_le_bytes(arr)))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(map_keyring_err(e)),
        }
    }

    fn store_commit_baseline(&self, vault_uuid: &str, counter: i64) -> Result<(), VaultError> {
        self.counter_entry(vault_uuid)?
            .set_secret(&counter.to_le_bytes())
            .map_err(map_keyring_err)
    }

    fn delete_commit_baseline(&self, vault_uuid: &str) -> Result<(), VaultError> {
        // Idempotent: a missing baseline is success (fresh device / crash-resume), unlike
        // `delete_secret_key`.
        match self.counter_entry(vault_uuid)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(map_keyring_err(e)),
        }
    }

    fn migrate_secret_key(
        &self,
        legacy_vault_id: &VaultId,
        vault_uuid: &str,
    ) -> Result<bool, VaultError> {
        let legacy = self.legacy_entry(legacy_vault_id)?;
        let mut bytes = match legacy.get_secret() {
            Ok(b) => b,
            Err(KeyringError::NoEntry) => return Ok(false), // nothing to migrate
            Err(e) => return Err(map_keyring_err(e)),
        };
        let key = <[u8; SECRET_KEY_LEN]>::try_from(bytes.as_slice()).map(Zeroizing::new);
        bytes.fill(0);
        let key = key.map_err(|_| VaultError::KeychainEntryNotFound)?;

        // Store under the uuid, then VERIFY it reads back BEFORE deleting the legacy entry
        // — the vault must stay unlockable if anything here fails.
        self.store_secret_key(vault_uuid, &key)?;
        let readback = self.read_secret_key(vault_uuid)?;
        if *readback != *key {
            return Err(VaultError::KeychainUnavailable);
        }
        match legacy.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(true),
            Err(e) => Err(map_keyring_err(e)),
        }
    }
}
