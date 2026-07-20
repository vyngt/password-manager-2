use crate::domain::shared::Timestamp;
use crate::domain::vault::kdf_params::KdfParams;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultConfig {
    pub id: String,
    pub magic: String,
    pub schema_version: i32,
    pub vault_salt: [u8; 32],
    pub kdf_params: KdfParams,
    pub verify_hash: [u8; 32],
    pub preferred_cipher_suite: i32,
    pub trash_retention_days: i32,
    pub audit_retention_days: i32,
    pub created_at: Timestamp,
    pub last_unlocked_at: Option<Timestamp>,
}
