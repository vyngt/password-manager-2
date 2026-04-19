//! DTOs for the Emergency Kit export + recovery commands.
//!
//! `EmergencyKitDto` is the structured content returned by
//! [`export_emergency_kit`](crate::commands::emergency_kit::export_emergency_kit).
//! The shell converts it to a PDF via `pdf::emergency_kit` (same crate).
//!
//! `RecoveryOutcomeDto` flattens
//! [`RecoveryOutcome`](vedge_core::RecoveryOutcome) — the session stays in
//! `AppState`, so the DTO carries only the side-channel bits (did the
//! keychain restore, and if not, why).

use serde::{Deserialize, Serialize};

use vedge_core::RecoveryOutcome;
use vedge_core::domain::vault::emergency_kit::EmergencyKitContent;

use crate::dto::common::ts_to_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyKitDto {
    pub vault_name: String,
    pub vault_path: String,
    /// RFC 3339 timestamp (millisecond precision, UTC).
    pub generated_at: String,
    pub secret_key_display: String,
    pub kdf_params_summary: String,
}

impl From<EmergencyKitContent> for EmergencyKitDto {
    fn from(c: EmergencyKitContent) -> Self {
        Self {
            vault_name: c.vault_name,
            vault_path: c.vault_path,
            generated_at: ts_to_string(c.generated_at),
            secret_key_display: c.secret_key_display,
            kdf_params_summary: c.kdf_params_summary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutcomeDto {
    pub keychain_restored: bool,
    #[serde(default)]
    pub keychain_error: Option<String>,
}

impl From<&RecoveryOutcome> for RecoveryOutcomeDto {
    fn from(o: &RecoveryOutcome) -> Self {
        Self {
            keychain_restored: o.keychain_restored,
            keychain_error: o.keychain_error.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockWithRecoveryKeyInputDto {
    pub vault_path: String,
    pub master_password: String,
    pub recovery_key_display: String,
}
