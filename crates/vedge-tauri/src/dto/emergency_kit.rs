//! Emergency Kit DTO conversion layer.

pub use vedge_ipc::{EmergencyKitDto, RecoveryOutcomeDto, UnlockWithRecoveryKeyInputDto};

use vedge_core::RecoveryOutcome;
use vedge_core::domain::vault::emergency_kit::EmergencyKitContent;

use crate::dto::common::ts_to_string;

#[must_use]
pub fn emergency_kit_to_dto(c: EmergencyKitContent) -> EmergencyKitDto {
    EmergencyKitDto {
        vault_name: c.vault_name,
        vault_path: c.vault_path,
        generated_at: ts_to_string(c.generated_at),
        secret_key_display: c.secret_key_display,
        kdf_params_summary: c.kdf_params_summary,
    }
}

#[must_use]
pub fn recovery_outcome_to_dto(o: &RecoveryOutcome) -> RecoveryOutcomeDto {
    RecoveryOutcomeDto {
        keychain_restored: o.keychain_restored,
        keychain_error: o.keychain_error.clone(),
    }
}
