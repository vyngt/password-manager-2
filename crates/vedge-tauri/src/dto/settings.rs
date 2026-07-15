//! App.db DTO conversion layer.

pub use vedge_ipc::{
    AppSettingDto, CreateCustomThemeInputDto, ExtensionSessionDto, KnownDeviceDto,
    RegisteredVaultDto, RegisteredVaultStatusDto, ThemeDto, UpdateCustomThemeInputDto,
};

use std::path::PathBuf;

use vedge_core::domain::app::entities::{
    AppSetting, ExtensionSession, KnownDevice, RegisteredVault, Theme,
};
use vedge_core::domain::shared::{DeviceId, SessionId, ThemeId};

use crate::dto::common::{b64_decode_fixed, b64_encode, ts_from_string, ts_to_string};
use crate::error::CommandError;

// ---- RegisteredVault -------------------------------------------------------------

#[must_use]
pub fn registered_vault_to_dto(r: &RegisteredVault) -> RegisteredVaultDto {
    RegisteredVaultDto {
        id: r.id.clone(),
        path: r.path.display().to_string(),
        display_name: r.display_name.clone(),
        last_opened: r.last_opened.map(ts_to_string),
        sort_order: r.sort_order,
    }
}

pub fn registered_vault_from_dto(dto: RegisteredVaultDto) -> Result<RegisteredVault, CommandError> {
    Ok(RegisteredVault {
        id: dto.id,
        path: PathBuf::from(dto.path),
        display_name: dto.display_name,
        last_opened: dto.last_opened.as_deref().map(ts_from_string).transpose()?,
        sort_order: dto.sort_order,
        // The recents DTO does not carry the uuid yet (populated in 5.2.2); the column
        // exists so a future record can fill it.
        vault_uuid: None,
    })
}

#[must_use]
pub fn registered_vault_status_to_dto(
    s: &vedge_core::RegisteredVaultStatus,
    openable: bool,
) -> RegisteredVaultStatusDto {
    RegisteredVaultStatusDto {
        vault: registered_vault_to_dto(&s.vault),
        exists: s.exists,
        openable,
    }
}

// ---- AppSetting --------------------------------------------------------------

#[must_use]
pub fn app_setting_to_dto(s: &AppSetting) -> AppSettingDto {
    AppSettingDto {
        key: s.key.clone(),
        value: s.value.clone(),
        updated_at: ts_to_string(s.updated_at),
    }
}

// ---- Theme -------------------------------------------------------------------

#[must_use]
pub fn theme_to_dto(t: &Theme) -> ThemeDto {
    ThemeDto {
        id: t.id.as_str().to_owned(),
        name: t.name.clone(),
        is_built_in: t.is_built_in,
        root_background: t.root_background.clone(),
        root_foreground: t.root_foreground.clone(),
        root_primary: t.root_primary.clone(),
        danger_base: t.danger_base.clone(),
        warning_base: t.warning_base.clone(),
        success_base: t.success_base.clone(),
        created_at: ts_to_string(t.created_at),
        updated_at: ts_to_string(t.updated_at),
    }
}

pub fn theme_from_dto(dto: ThemeDto) -> Result<Theme, CommandError> {
    Ok(Theme {
        id: ThemeId::from_raw(dto.id),
        name: dto.name,
        is_built_in: dto.is_built_in,
        root_background: dto.root_background,
        root_foreground: dto.root_foreground,
        root_primary: dto.root_primary,
        danger_base: dto.danger_base,
        warning_base: dto.warning_base,
        success_base: dto.success_base,
        created_at: ts_from_string(&dto.created_at)?,
        updated_at: ts_from_string(&dto.updated_at)?,
    })
}

// ---- KnownDevice -------------------------------------------------------------

#[must_use]
pub fn known_device_to_dto(d: &KnownDevice) -> KnownDeviceDto {
    KnownDeviceDto {
        device_id: d.device_id.as_str().to_owned(),
        display_name: d.display_name.clone(),
        public_key_b64: b64_encode(&d.public_key),
        first_seen: ts_to_string(d.first_seen),
        last_seen: d.last_seen.map(ts_to_string),
    }
}

pub fn known_device_from_dto(dto: KnownDeviceDto) -> Result<KnownDevice, CommandError> {
    Ok(KnownDevice {
        device_id: DeviceId::from_raw(dto.device_id),
        display_name: dto.display_name,
        public_key: b64_decode_fixed::<32>(&dto.public_key_b64)?,
        first_seen: ts_from_string(&dto.first_seen)?,
        last_seen: dto.last_seen.as_deref().map(ts_from_string).transpose()?,
    })
}

// ---- ExtensionSession --------------------------------------------------------

#[must_use]
pub fn extension_session_to_dto(s: &ExtensionSession) -> ExtensionSessionDto {
    ExtensionSessionDto {
        session_id: s.session_id.as_str().to_owned(),
        browser: s.browser.clone(),
        profile_name: s.profile_name.clone(),
        session_key_b64: b64_encode(&s.session_key),
        created_at: ts_to_string(s.created_at),
        last_active_at: s.last_active_at.map(ts_to_string),
    }
}

pub fn extension_session_from_dto(
    dto: ExtensionSessionDto,
) -> Result<ExtensionSession, CommandError> {
    Ok(ExtensionSession {
        session_id: SessionId::from_raw(dto.session_id),
        browser: dto.browser,
        profile_name: dto.profile_name,
        session_key: b64_decode_fixed::<32>(&dto.session_key_b64)?,
        created_at: ts_from_string(&dto.created_at)?,
        last_active_at: dto
            .last_active_at
            .as_deref()
            .map(ts_from_string)
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    fn fixed_ts() -> vedge_ipc::Timestamp {
        Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap()
    }

    #[test]
    fn theme_round_trip_preserves_fields() {
        let t = Theme {
            id: ThemeId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            name: "Midnight".into(),
            is_built_in: true,
            root_background: "#000".into(),
            root_foreground: "#eee".into(),
            root_primary: "#4af".into(),
            danger_base: Some("#f44".into()),
            warning_base: None,
            success_base: None,
            created_at: fixed_ts(),
            updated_at: fixed_ts(),
        };
        let dto = theme_to_dto(&t);
        let back = theme_from_dto(dto).unwrap();
        assert_eq!(back.id, t.id);
        assert_eq!(back.name, t.name);
        assert_eq!(back.is_built_in, t.is_built_in);
        assert_eq!(
            back.created_at.timestamp_millis(),
            t.created_at.timestamp_millis()
        );
    }

    #[test]
    fn known_device_round_trips_public_key() {
        let pk: [u8; 32] = std::array::from_fn(|i| u8::try_from(i & 0xff).unwrap());
        let d = KnownDevice {
            device_id: DeviceId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            display_name: "laptop".into(),
            public_key: pk,
            first_seen: fixed_ts(),
            last_seen: None,
        };
        let dto = known_device_to_dto(&d);
        let back = known_device_from_dto(dto).unwrap();
        assert_eq!(back.public_key, pk);
    }

    #[test]
    fn extension_session_rejects_wrong_key_length() {
        let dto = ExtensionSessionDto {
            session_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
            browser: "firefox".into(),
            profile_name: None,
            session_key_b64: b64_encode(&[0u8; 16]),
            created_at: "2026-01-01T00:00:00.000Z".into(),
            last_active_at: None,
        };
        let err = extension_session_from_dto(dto).unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }
}
