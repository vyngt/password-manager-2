//! `CommandError` — the single error type Tauri sees.
//!
//! Wraps `VaultError` / `AppDbError` via `From` impls and tags variants with
//! `#[serde(tag = "kind", content = "message")]` so the JS side can
//! discriminate without parsing strings. Auth failures deliberately collapse
//! so the frontend can't distinguish wrong-key from tampered-ciphertext.
//!
//! **Never embed raw secret bytes in `message`.** All domain errors upstream
//! already avoid this; the `From` impls below drop underlying sources that
//! might carry sensitive detail.

use vedge_core::domain::app::errors::AppDbError;
use vedge_core::domain::shared::StorageError;
use vedge_core::domain::vault::errors::VaultError;
use vedge_ipc::IpcError;

#[derive(Debug, serde::Serialize, thiserror::Error)]
#[serde(tag = "kind", content = "message")]
pub enum CommandError {
    /// Master password or Secret Key didn't match. Shown in the unlock screen.
    #[error("wrong password or Secret Key")]
    WrongCredentials,

    /// Generic "vault can't currently decrypt this row". Covers tampered
    /// ciphertext, wrong AAD, wrong DEK — distinguishing them would leak
    /// information.
    #[error("decryption failed")]
    Decryption,

    /// Referenced entry / tag / folder doesn't exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// Caller asked for a session that isn't in the registry (either never
    /// unlocked or already locked).
    #[error("vault is not open: {0}")]
    VaultNotOpen(String),

    /// OS keychain was unreachable, locked, denied, or missing the entry.
    /// The specific `VaultError` variant is collapsed here — shells surface
    /// a recovery flow rather than trying to distinguish.
    #[error("keychain error: {0}")]
    Keychain(String),

    /// Persistence layer failure (`SQLite`, filesystem).
    #[error("storage failure: {0}")]
    Storage(String),

    /// Caller-supplied input was malformed (bad ULID, bad base64, enum
    /// mismatch, etc.).
    #[error("invalid input: {0}")]
    Invalid(String),

    /// Catch-all for unexpected internal states. Distinct from `Storage` so
    /// the frontend can treat it as a bug rather than a transient failure.
    #[error("internal error")]
    Internal,
}

impl From<VaultError> for CommandError {
    fn from(e: VaultError) -> Self {
        match e {
            VaultError::WrongCredentials => Self::WrongCredentials,

            // All authentication-style failures collapse — deliberately
            // indistinguishable to JS. See the CryptoProvider docstring.
            VaultError::DecryptionFailed => Self::Decryption,
            VaultError::EncryptionFailed | VaultError::MlockFailed => Self::Internal,

            VaultError::EntryNotFound(id) => Self::NotFound(format!("entry {id}")),
            VaultError::TagNotFound(id) => Self::NotFound(format!("tag {id}")),
            VaultError::FolderNotFound(id) => Self::NotFound(format!("folder {id}")),
            VaultError::BlobNotFound(id) => Self::NotFound(format!("blob {id}")),
            VaultError::AuditNotFound(id) => Self::NotFound(format!("audit {id}")),

            VaultError::KeychainUnavailable
            | VaultError::KeychainAccessDenied
            | VaultError::KeychainEntryNotFound => Self::Keychain(e.to_string()),

            VaultError::Storage(inner) => Self::Storage(storage_detail(&inner)),

            VaultError::ConfigMissing
            | VaultError::BadMagic
            | VaultError::UnsupportedSchemaVersion(_)
            | VaultError::UnknownAuditAction(_)
            | VaultError::InvalidKdfParams(_)
            | VaultError::InvalidBlobLength { .. }
            | VaultError::InvalidEntryId(_)
            | VaultError::InvalidTagId(_)
            | VaultError::UnsupportedPayloadSchema(_)
            | VaultError::UnsupportedEntryType(_)
            | VaultError::MalformedPayload(_)
            | VaultError::FolderNotEmpty
            | VaultError::DocumentTooLarge { .. }
            | VaultError::FieldNotApplicable
            | VaultError::KeyDerivationFailed(_)
            | VaultError::InvalidRecoveryKey(_) => Self::Invalid(e.to_string()),
        }
    }
}

impl From<StorageError> for CommandError {
    fn from(e: StorageError) -> Self {
        Self::Storage(storage_detail(&e))
    }
}

impl From<AppDbError> for CommandError {
    fn from(e: AppDbError) -> Self {
        match e {
            AppDbError::RecentVaultNotFound(id) => Self::NotFound(format!("recent_vault {id}")),
            AppDbError::SettingNotFound(key) => Self::NotFound(format!("setting {key}")),
            AppDbError::ThemeNotFound(id) => Self::NotFound(format!("theme {id}")),
            AppDbError::DeviceNotFound(id) => Self::NotFound(format!("device {id}")),
            AppDbError::ExtensionSessionNotFound(id) => {
                Self::NotFound(format!("extension_session {id}"))
            }
            AppDbError::BuiltInThemeImmutable => {
                Self::Invalid("built-in themes cannot be modified or deleted".into())
            }
            AppDbError::InvalidSettingValue { key, reason } => {
                Self::Invalid(format!("{key}: {reason}"))
            }
            AppDbError::Storage(inner) => Self::Storage(storage_detail(&inner)),
        }
    }
}

fn storage_detail(e: &StorageError) -> String {
    // `StorageError::Display` is already safe — never echoes user data.
    e.to_string()
}

impl From<IpcError> for CommandError {
    fn from(e: IpcError) -> Self {
        // All IPC wire-format errors — bad timestamps, malformed base64,
        // wrong byte length — are caller-input errors from the frontend's
        // perspective.
        Self::Invalid(e.to_string())
    }
}
