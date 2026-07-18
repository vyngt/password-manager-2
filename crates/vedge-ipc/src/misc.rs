//! Misc DTOs: field selector, maintenance report, document export,
//! unlock/create-vault input, change-password input.

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
    // ---- 5.4.1 (Complete the Reveal Door) ----
    Pin,
    PrivateKey,
    Passphrase,
    ApiSecret,
    NationalId,
    RecoveryCode(u32),
    Custom(String),
}

// ---- EnvExportFormatDto ------------------------------------------------------

/// Which shape the env-var **set** copy/reveal produces (slice 5.4.1 ⑥). Carries
/// no data, so a plain C-like enum (unlike the adjacently-tagged
/// [`FieldSelectorDto`]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EnvExportFormatDto {
    DotEnv,
    Json,
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

// ---- UnlockResult DTO --------------------------------------------------------

/// The result of a successful unlock (slice 5.2c).
///
/// Advisory, non-secret: if the vault's `commit_counter` was below this device's
/// keychain baseline, `rollback_detected` is `true` and `rollback_delta` carries
/// `baseline − file`. The shell raises a warning toast; the session unlocks regardless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockResultDto {
    pub rollback_detected: bool,
    #[serde(default)]
    pub rollback_delta: Option<i64>,
}

// ---- ChangePassword input DTO ------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordInputDto {
    /// The current master password — re-verified against the live session's
    /// `verify_hash` before any rewrap (slice 5.6 ④).
    pub current_password: String,
    pub new_password: String,
    #[serde(default)]
    pub new_secret_key_b64: Option<String>,
}

/// The re-issued Secret Key after a rotation — shown once, never persisted in
/// plaintext (the same show-once contract as `CreateVaultOutputDto`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretKeyRotationOutputDto {
    pub secret_key_display: String,
}

// ---- CreateVault DTOs --------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultInputDto {
    pub vault_path: String,
    pub master_password: String,
    /// `None` → the core generates a fresh 16-byte Secret Key.
    #[serde(default)]
    pub secret_key_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultOutputDto {
    /// `A3-XXXXX-…` Emergency-Kit string. Shown once; never stored.
    pub secret_key_display: String,
    pub keychain_stored: bool,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn create_vault_input_round_trips_with_secret_key() {
        let dto = CreateVaultInputDto {
            vault_path: "/tmp/new.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: Some("AAAAAAAAAAAAAAAAAAAAAA==".into()),
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: CreateVaultInputDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.vault_path, dto.vault_path);
        assert_eq!(back.master_password, dto.master_password);
        assert_eq!(back.secret_key_b64, dto.secret_key_b64);
    }

    #[test]
    fn create_vault_input_defaults_missing_secret_key() {
        let back: CreateVaultInputDto =
            serde_json::from_str(r#"{"vault_path":"/tmp/x.vdb","master_password":"pw"}"#).unwrap();
        assert!(back.secret_key_b64.is_none());
    }

    #[test]
    fn create_vault_output_round_trips() {
        let dto = CreateVaultOutputDto {
            secret_key_display: "A3-ABCDE-FGHIJ".into(),
            keychain_stored: true,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: CreateVaultOutputDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.secret_key_display, dto.secret_key_display);
        assert_eq!(back.keychain_stored, dto.keychain_stored);
    }
}
