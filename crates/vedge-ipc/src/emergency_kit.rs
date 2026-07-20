//! Emergency Kit wire types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyKitDto {
    pub vault_name: String,
    pub vault_path: String,
    /// RFC 3339 timestamp (millisecond precision, UTC).
    pub generated_at: String,
    pub secret_key_display: String,
    pub kdf_params_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutcomeDto {
    pub keychain_restored: bool,
    #[serde(default)]
    pub keychain_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockWithRecoveryKeyInputDto {
    pub vault_path: String,
    pub master_password: String,
    pub recovery_key_display: String,
}
