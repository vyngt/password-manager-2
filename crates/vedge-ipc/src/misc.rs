//! Misc DTOs: field selector, maintenance report, document export,
//! unlock input, change-password input.

use serde::{Deserialize, Serialize};

// ---- FieldSelectorDto --------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum FieldSelectorDto {
    Password,
    Username,
    TotpCode,
    CardNumber,
    Cvv,
    ApiKey,
    EnvVar(String),
    Custom(String),
}

// ---- MaintenanceReportDto ----------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct MaintenanceReportDto {
    pub trashed_entries_deleted: u64,
    pub audit_events_deleted: u64,
    pub orphaned_blobs_deleted: u64,
}

// ---- ExportedDocumentDto -----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedDocumentDto {
    pub filename: String,
    /// Base64-encoded document bytes.
    pub content_b64: String,
}

// ---- UnlockVault input DTO ---------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockVaultInputDto {
    pub vault_path: String,
    pub master_password: String,
    #[serde(default)]
    pub secret_key_b64: Option<String>,
}

// ---- ChangePassword input DTO ------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordInputDto {
    pub new_password: String,
    #[serde(default)]
    pub new_secret_key_b64: Option<String>,
}
