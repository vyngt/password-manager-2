//! One-off DTOs that don't fit the common / entry / tag / settings buckets:
//! field-selector for `copy_field`, the `run_maintenance` report, the
//! `export_document` output, and the unlock-vault wire input.

use serde::{Deserialize, Serialize};

use vedge_core::MaintenanceReport;
use vedge_core::application::vault::use_cases::FieldSelector;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use zeroize::Zeroizing;

use crate::dto::common::b64_decode_fixed;
use crate::error::CommandError;

// ---- FieldSelectorDto --------------------------------------------------------

/// Which field `copy_field` should pull onto the clipboard. Adjacently
/// tagged so the variants carrying data (`EnvVar`, `Custom`) stay
/// discriminable on the JS side.
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

impl From<FieldSelectorDto> for FieldSelector {
    fn from(v: FieldSelectorDto) -> Self {
        match v {
            FieldSelectorDto::Password => Self::Password,
            FieldSelectorDto::Username => Self::Username,
            FieldSelectorDto::TotpCode => Self::TotpCode,
            FieldSelectorDto::CardNumber => Self::CardNumber,
            FieldSelectorDto::Cvv => Self::Cvv,
            FieldSelectorDto::ApiKey => Self::ApiKey,
            FieldSelectorDto::EnvVar(k) => Self::EnvVar(k),
            FieldSelectorDto::Custom(k) => Self::Custom(k),
        }
    }
}

// ---- MaintenanceReportDto ----------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)] // "_deleted" suffix carries meaning
pub struct MaintenanceReportDto {
    pub trashed_entries_deleted: u64,
    pub audit_events_deleted: u64,
    pub orphaned_blobs_deleted: u64,
}

impl From<MaintenanceReport> for MaintenanceReportDto {
    fn from(r: MaintenanceReport) -> Self {
        Self {
            trashed_entries_deleted: r.trashed_entries_deleted,
            audit_events_deleted: r.audit_events_deleted,
            orphaned_blobs_deleted: r.orphaned_blobs_deleted,
        }
    }
}

// ---- ExportedDocumentDto -----------------------------------------------------

/// Output of `export_document`. Binary content is base64 to stay inside the
/// Tauri JSON-only IPC without encoding weirdness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedDocumentDto {
    pub filename: String,
    /// Base64-encoded document bytes.
    pub content_b64: String,
}

// ---- UnlockVault input DTO ---------------------------------------------------

/// Input to the `unlock_vault` command.
///
/// `master_password` is a plain `String` on the wire; the command body
/// wraps it in `Zeroizing` before calling the use case. `secret_key_b64`
/// is optional — when `None`, the use case reads the key from the OS
/// keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockVaultInputDto {
    pub vault_path: String,
    pub master_password: String,
    #[serde(default)]
    pub secret_key_b64: Option<String>,
}

impl UnlockVaultInputDto {
    /// Decode the optional secret key into the zeroizing fixed-length
    /// form the use case expects.
    pub fn decode_secret_key(
        &self,
    ) -> Result<Option<Zeroizing<[u8; SECRET_KEY_LEN]>>, CommandError> {
        self.secret_key_b64
            .as_deref()
            .map(|s| b64_decode_fixed::<SECRET_KEY_LEN>(s).map(Zeroizing::new))
            .transpose()
    }
}

// ---- ChangePassword input DTO ------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordInputDto {
    pub new_password: String,
    #[serde(default)]
    pub new_secret_key_b64: Option<String>,
}

impl ChangePasswordInputDto {
    pub fn decode_secret_key(
        &self,
    ) -> Result<Option<Zeroizing<[u8; SECRET_KEY_LEN]>>, CommandError> {
        self.new_secret_key_b64
            .as_deref()
            .map(|s| b64_decode_fixed::<SECRET_KEY_LEN>(s).map(Zeroizing::new))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::dto::common::b64_encode;

    #[test]
    fn field_selector_round_trips_envvar_payload() {
        let dto = FieldSelectorDto::EnvVar("DATABASE_URL".into());
        let json = serde_json::to_string(&dto).unwrap();
        let back: FieldSelectorDto = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, FieldSelectorDto::EnvVar(ref k) if k == "DATABASE_URL"));
    }

    #[test]
    fn unlock_input_decodes_secret_key() {
        let key = [0x11u8; SECRET_KEY_LEN];
        let dto = UnlockVaultInputDto {
            vault_path: "/tmp/x.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: Some(b64_encode(&key)),
        };
        let decoded = dto.decode_secret_key().unwrap().unwrap();
        assert_eq!(*decoded, key);
    }

    #[test]
    fn unlock_input_accepts_missing_secret_key() {
        let dto = UnlockVaultInputDto {
            vault_path: "/tmp/x.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: None,
        };
        assert!(dto.decode_secret_key().unwrap().is_none());
    }
}
