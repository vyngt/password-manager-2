//! Misc DTO conversion layer.

pub use vedge_ipc::{
    ChangePasswordInputDto, CreateVaultInputDto, CreateVaultOutputDto, ExportedDocumentDto,
    FieldSelectorDto, MaintenanceReportDto, UnlockVaultInputDto,
};

use vedge_core::MaintenanceReport;
use vedge_core::application::vault::use_cases::FieldSelector;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use zeroize::Zeroizing;

use crate::dto::common::b64_decode_fixed;
use crate::error::CommandError;

// ---- FieldSelectorDto → FieldSelector ----------------------------------------

#[must_use]
pub fn field_selector_from_dto(v: FieldSelectorDto) -> FieldSelector {
    match v {
        FieldSelectorDto::Password => FieldSelector::Password,
        FieldSelectorDto::Username => FieldSelector::Username,
        FieldSelectorDto::TotpCode => FieldSelector::TotpCode,
        FieldSelectorDto::CardNumber => FieldSelector::CardNumber,
        FieldSelectorDto::Cvv => FieldSelector::Cvv,
        FieldSelectorDto::ApiKey => FieldSelector::ApiKey,
        FieldSelectorDto::EnvVar(k) => FieldSelector::EnvVar(k),
        FieldSelectorDto::Custom(k) => FieldSelector::Custom(k),
    }
}

// ---- MaintenanceReport → DTO -------------------------------------------------

#[must_use]
pub const fn maintenance_report_to_dto(r: MaintenanceReport) -> MaintenanceReportDto {
    MaintenanceReportDto {
        trashed_entries_deleted: r.trashed_entries_deleted,
        audit_events_deleted: r.audit_events_deleted,
        orphaned_blobs_deleted: r.orphaned_blobs_deleted,
    }
}

// ---- UnlockVaultInputDto helpers ---------------------------------------------

/// Decode the optional secret key into the zeroizing fixed-length form the
/// use case expects.
pub fn decode_unlock_secret_key(
    dto: &UnlockVaultInputDto,
) -> Result<Option<Zeroizing<[u8; SECRET_KEY_LEN]>>, CommandError> {
    dto.secret_key_b64
        .as_deref()
        .map(|s| b64_decode_fixed::<SECRET_KEY_LEN>(s).map(Zeroizing::new))
        .transpose()
}

pub fn decode_change_password_secret_key(
    dto: &ChangePasswordInputDto,
) -> Result<Option<Zeroizing<[u8; SECRET_KEY_LEN]>>, CommandError> {
    dto.new_secret_key_b64
        .as_deref()
        .map(|s| b64_decode_fixed::<SECRET_KEY_LEN>(s).map(Zeroizing::new))
        .transpose()
}

/// Decode the optional Secret Key for vault creation. `None` → the core
/// generates a fresh one. Mirrors [`decode_unlock_secret_key`].
pub fn decode_create_secret_key(
    dto: &CreateVaultInputDto,
) -> Result<Option<Zeroizing<[u8; SECRET_KEY_LEN]>>, CommandError> {
    dto.secret_key_b64
        .as_deref()
        .map(|s| b64_decode_fixed::<SECRET_KEY_LEN>(s).map(Zeroizing::new))
        .transpose()
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
        let decoded = decode_unlock_secret_key(&dto).unwrap().unwrap();
        assert_eq!(*decoded, key);
    }

    #[test]
    fn unlock_input_accepts_missing_secret_key() {
        let dto = UnlockVaultInputDto {
            vault_path: "/tmp/x.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: None,
        };
        assert!(decode_unlock_secret_key(&dto).unwrap().is_none());
    }

    #[test]
    fn create_input_decodes_secret_key() {
        let key = [0x22u8; SECRET_KEY_LEN];
        let dto = CreateVaultInputDto {
            vault_path: "/tmp/new.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: Some(b64_encode(&key)),
        };
        let decoded = decode_create_secret_key(&dto).unwrap().unwrap();
        assert_eq!(*decoded, key);
    }

    #[test]
    fn create_input_accepts_missing_secret_key() {
        let dto = CreateVaultInputDto {
            vault_path: "/tmp/new.vdb".into(),
            master_password: "hunter2".into(),
            secret_key_b64: None,
        };
        assert!(decode_create_secret_key(&dto).unwrap().is_none());
    }
}
