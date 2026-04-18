use thiserror::Error;

use crate::domain::shared::{EntryId, StorageError, TagId};

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("vault config not found")]
    ConfigMissing,

    #[error("vault file is not a Vedge vault (magic mismatch)")]
    BadMagic,

    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(i32),

    #[error("entry not found: {0}")]
    EntryNotFound(EntryId),

    #[error("tag not found: {0}")]
    TagNotFound(TagId),

    #[error("audit event not found: {0}")]
    AuditNotFound(String),

    #[error("unknown audit action: {0}")]
    UnknownAuditAction(String),

    #[error("invalid kdf params: {0}")]
    InvalidKdfParams(String),

    #[error("invalid blob length for {field} (expected {expected}, got {actual})")]
    InvalidBlobLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("invalid entry id: {0}")]
    InvalidEntryId(String),

    #[error("invalid tag id: {0}")]
    InvalidTagId(String),

    #[error("unsupported payload schema: {0}")]
    UnsupportedPayloadSchema(u32),

    #[error("unsupported entry type: {0}")]
    UnsupportedEntryType(String),

    #[error("malformed payload: {0}")]
    MalformedPayload(String),

    // --- crypto ---
    #[error("wrong password or Secret Key")]
    WrongCredentials,

    #[error("decryption failed")]
    DecryptionFailed,

    #[error("encryption failed")]
    EncryptionFailed,

    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("mlock() failed — cannot pin key material to RAM")]
    MlockFailed,

    // --- keychain ---
    #[error("OS keychain is unavailable")]
    KeychainUnavailable,

    #[error("OS keychain access denied")]
    KeychainAccessDenied,

    #[error("Secret Key not found in keychain")]
    KeychainEntryNotFound,

    #[error("storage failure")]
    Storage(#[source] StorageError),
}

impl From<StorageError> for VaultError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
