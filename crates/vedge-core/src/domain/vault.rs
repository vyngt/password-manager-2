pub mod aad;
pub mod crypto_constants;
pub mod emergency_kit;
pub mod entities;
pub mod env_export;
pub mod errors;
pub mod health;
pub mod index;
pub mod kdf_params;
pub mod otpauth;
pub mod payloads;
pub mod secret_key;
pub mod secret_update;
pub mod totp;

pub use aad::{blob_aad, entry_aad, tag_aad};
pub use crypto_constants::{
    BLOB_AAD_SUFFIX, DEK_LEN, DEK_WRAPPED_LEN, HKDF_INFO_2SKD, HKDF_INFO_KEK, HKDF_INFO_SYNC_AUTH,
    HKDF_INFO_VERIFY, KEK_LEN, MASTER_KEY_LEN, NONCE_LEN, SECRET_KEY_LEN, VAULT_SALT_LEN,
    VERIFY_HASH_LEN,
};
pub use emergency_kit::EmergencyKitContent;
pub use entities::{AuditAction, AuditEvent, EntryRow, TagRow, VaultConfig};
pub use errors::VaultError;
pub use health::{
    AgeConfidence, Finding, FindingKind, HealthReport, HealthScanInput, HealthSummary, SecretField,
    Severity, SkipReason, Skipped, WeakPolicy,
};
pub use index::{IndexEntry, TagMeta, VaultIndex};
pub use kdf_params::KdfParams;
pub use otpauth::{TotpEnrolment, parse_totp_input};
pub use payloads::{
    Address, ApiKeyPayload, CURRENT_PAYLOAD_SCHEMA, CardPayload, CommonMeta, DocumentPayload,
    EntryPayload, EntryType, EnvVar, EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload,
    NotePayload, SshKeyPayload, TagPayload, UnknownPayload,
};
pub use secret_key::{
    RECOVERY_KEY_FORMAT_PREFIX, SECRET_KEY_FORMAT_PREFIX, format_recovery_key, format_secret_key,
    parse_recovery_key, parse_secret_key,
};
pub use secret_update::{
    EnvVarUpdate, SecretListUpdate, SecretUpdate, SecretUpdates, resolve_secrets,
};
pub use totp::{TotpAlgorithm, TotpCode, TotpParams, TotpUpdate};
