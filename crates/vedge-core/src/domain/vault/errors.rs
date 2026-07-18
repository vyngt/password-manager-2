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

    // --- TOTP (slice 4.2) ---
    #[error("invalid TOTP parameters: {0}")]
    InvalidTotpParams(String),

    #[error("HOTP is not supported — only time-based (TOTP) codes")]
    HotpNotSupported,

    #[error("otpauth-migration:// import is not supported yet")]
    TotpMigrationNotSupported,

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
    InvalidSecretKey(String),

    // --- keychain ---
    #[error("OS keychain is unavailable")]
    KeychainUnavailable,

    #[error("OS keychain access denied")]
    KeychainAccessDenied,

    #[error("Secret Key not found in keychain")]
    KeychainEntryNotFound,

    // --- biometric ---
    #[error("biometric authentication is unavailable on this device")]
    BiometricUnavailable,

    #[error("no biometric enrollment for this vault")]
    BiometricNotEnrolled,

    #[error("biometric prompt was cancelled")]
    BiometricCancelled,

    #[error("biometric authenticator failure: {0}")]
    BiometricFailed(String),

    // --- breach (slice 4.4) ---
    #[error("breach lookup failed: {0}")]
    BreachLookup(String),

    // --- snapshots (slice 5.2.1) — dedicated variants, not a stringly `MalformedPayload` ---
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("snapshot manifest is unreadable: {0}")]
    SnapshotManifestUnreadable(String),

    #[error("unsupported snapshot format version: {0}")]
    SnapshotUnsupportedFormat(u32),

    #[error("snapshot is corrupt: {0}")]
    SnapshotCorrupt(String),

    /// Shared by `revert_to_snapshot` (5.2.1) and `replace_vault_from_backup` (5.2.2) —
    /// both roll the vault back to an earlier state and both gate on the same explicit yes.
    #[error("this would roll the vault back to an earlier state; confirm to proceed")]
    RollbackNotConfirmed,

    // --- backup: open / replace (slice 5.2.2) — dedicated variants, not a stringly `MalformedPayload` ---
    /// A backup written by a NEWER build. An OLDER `format_version` is always accepted
    /// (finding H1): once a `.vbk` exists on a user's disk, every future version must
    /// read it — forever.
    #[error("unsupported backup format version: {0}")]
    BackupUnsupportedFormat(u32),

    /// 🔴 "Open backup" never overwrites. There is no flag and no override — overwriting
    /// is a different operation, with a different name and a different button.
    #[error("something already exists at the destination: {0}")]
    DestinationOccupied(String),

    #[error("this backup belongs to a different vault ({backup} != {target})")]
    BackupWrongVault { backup: String, target: String },

    #[error(
        "this backup was made with a different master password or Secret Key; confirm to proceed"
    )]
    BackupCredentialsDiffer,

    /// The target could not be read, so its identity could not be checked against the
    /// backup's. Its own acknowledgement — never a silent skip (finding H2).
    #[error(
        "the target vault could not be read, so it could not be identified; confirm to proceed"
    )]
    TargetUnverified,

    #[error("there is no vault to replace at this path — open the backup instead")]
    TargetMissing,

    // --- export / import entries (slice 5.3) ---
    /// An export sealed by a NEWER build. An OLDER `format_version` is always
    /// accepted (H1): once one export exists on a user's disk, every future
    /// version must read it — forever. Guarded with `>`, never `!=`.
    #[error("unsupported export format version: {0}")]
    ExportUnsupportedFormat(u32),

    /// The export file's header is not a `VEdge` export (bad magic, truncated, or
    /// structurally invalid) — distinct from a wrong password (`DecryptionFailed`).
    #[error("export file is malformed: {0}")]
    ExportMalformed(String),

    /// `commit_import` / `cancel_import` was called with no import staged in this
    /// session — begin the import first (slice 5.3b).
    #[error("no import is in progress for this session")]
    ImportNotStarted,

    /// A snapshot whose `verify_hash_prefix` no longer matches the live vault: a
    /// credential change since it was taken failed to rewrap it (⑬), so its DEKs are
    /// under an older KEK this session cannot unwrap. Recovering entries from it (the
    /// tweezers) needs the password of its own moment — surfaced, never a silent skip
    /// (slice 5.3c).
    #[error(
        "this snapshot predates a credential change; recovering from it needs its original password"
    )]
    SnapshotStale,

    // --- sealed secrets (slice 5.4 — the WASM-Secret Sentinel) ---
    /// A required secret field (login password, card number/cvv, ssh private key,
    /// api key) received `Clear`, or `Unchanged` on a create with nothing to carry
    /// forward. A required credential can never be emptied — enforced in the use
    /// case, not the form.
    #[error("required secret field cannot be cleared: {0}")]
    RequiredSecretCleared(&'static str),

    /// Two inbound env-var rows share a key. Keys are the schema; they must be
    /// unique.
    #[error("duplicate env-var key: {0}")]
    DuplicateEnvVarKey(String),

    /// An env-var row carried `Unchanged` for a key with no stored value to carry
    /// forward (a new key must `Set` its value).
    #[error("env var '{0}' is unchanged but has no stored value")]
    EnvVarUnchangedWithoutStored(String),

    /// A row that `.env` cannot represent (slice 5.4.1 ⑥): a **value** with a
    /// newline (no portable `.env` encoding), or a **key** that is not a shell
    /// identifier (`[A-Za-z_][A-Za-z0-9_]*`) — a key with `=`, whitespace, or a
    /// newline would split wrong or inject a line, the exact hazard the value-side
    /// single-quoting guards against. The set-copy's `DotEnv` path rejects it so
    /// the UI can point at JSON (the lossless format) rather than emit corrupt or
    /// injected output. The `key` names the offending row.
    #[error("env var '{key}' cannot be exported as .env; use JSON")]
    EnvValueNotDotEnvSafe { key: String },

    // --- screen lock (slice 4.5b) ---
    #[error("screen-lock state is unavailable on this device")]
    ScreenLockUnavailable,

    #[error("storage failure")]
    Storage(#[source] StorageError),
}

impl From<StorageError> for VaultError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
