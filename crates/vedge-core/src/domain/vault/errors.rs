use thiserror::Error;

use crate::domain::shared::{EntryId, StorageError, TagId};

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("vault config not found")]
    ConfigMissing,

    #[error("vault file is not a Vedge vault (magic mismatch)")]
    BadMagic,

    #[error("a vault already exists at this path")]
    VaultAlreadyExists,

    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(i32),

    #[error("entry not found: {0}")]
    EntryNotFound(EntryId),

    #[error("tag not found: {0}")]
    TagNotFound(TagId),

    #[error("audit event not found: {0}")]
    AuditNotFound(String),

    #[error("history snapshot not found: {0}")]
    HistoryNotFound(String),

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

    #[error("folder not found: {0}")]
    FolderNotFound(EntryId),

    #[error("folder has children; move or delete them first")]
    FolderNotEmpty,

    #[error("document too large: {size} bytes exceeds {limit} bytes")]
    DocumentTooLarge { size: u64, limit: u64 },

    #[error("blob not found for entry {0}")]
    BlobNotFound(EntryId),

    #[error("requested field is not present on this entry type")]
    FieldNotApplicable,

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

    #[error("invalid recovery key: {0}")]
    InvalidRecoveryKey(String),

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
