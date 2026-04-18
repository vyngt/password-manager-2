//! App.db DTOs: recent vaults, settings, themes, known devices, extension
//! sessions. Binary fields (`public_key`, `session_key`) cross as base64.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use vedge_core::domain::app::entities::{
    AppSetting, ExtensionSession, KnownDevice, RecentVault, Theme,
};
use vedge_core::domain::shared::{DeviceId, SessionId, ThemeId};

use crate::dto::common::{b64_decode_fixed, b64_encode, ts_from_string, ts_to_string};
use crate::error::CommandError;

// ---- RecentVault -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentVaultDto {
    pub id: String,
    /// Path as a string — platform-native separator. Shell re-wraps to
    /// `PathBuf` on input.
    pub path: String,
    pub display_name: String,
    #[serde(default)]
    pub last_opened: Option<String>,
    pub sort_order: i32,
}

impl From<&RecentVault> for RecentVaultDto {
    fn from(r: &RecentVault) -> Self {
        Self {
            id: r.id.clone(),
            path: r.path.display().to_string(),
            display_name: r.display_name.clone(),
            last_opened: r.last_opened.map(ts_to_string),
            sort_order: r.sort_order,
        }
    }
}

impl RecentVaultDto {
    pub fn into_domain(self) -> Result<RecentVault, CommandError> {
        Ok(RecentVault {
            id: self.id,
            path: PathBuf::from(self.path),
            display_name: self.display_name,
            last_opened: self.last_opened.as_deref().map(ts_from_string).transpose()?,
            sort_order: self.sort_order,
        })
    }
}

// ---- AppSetting --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingDto {
    pub key: String,
    /// Stored as raw JSON — any shape the caller wants.
    pub value: Value,
    pub updated_at: String,
}

impl From<&AppSetting> for AppSettingDto {
    fn from(s: &AppSetting) -> Self {
        Self {
            key: s.key.clone(),
            value: s.value.clone(),
            updated_at: ts_to_string(s.updated_at),
        }
    }
}

// ---- Theme -------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDto {
    pub id: String,
    pub name: String,
    pub is_built_in: bool,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    #[serde(default)]
    pub danger_base: Option<String>,
    #[serde(default)]
    pub warning_base: Option<String>,
    #[serde(default)]
    pub success_base: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Theme> for ThemeDto {
    fn from(t: &Theme) -> Self {
        Self {
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
}

impl ThemeDto {
    pub fn into_domain(self) -> Result<Theme, CommandError> {
        Ok(Theme {
            id: ThemeId::from_raw(self.id),
            name: self.name,
            is_built_in: self.is_built_in,
            root_background: self.root_background,
            root_foreground: self.root_foreground,
            root_primary: self.root_primary,
            danger_base: self.danger_base,
            warning_base: self.warning_base,
            success_base: self.success_base,
            created_at: ts_from_string(&self.created_at)?,
            updated_at: ts_from_string(&self.updated_at)?,
        })
    }
}

// ---- KnownDevice -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownDeviceDto {
    pub device_id: String,
    pub display_name: String,
    /// Base64 of the 32-byte X25519 public key.
    pub public_key_b64: String,
    pub first_seen: String,
    #[serde(default)]
    pub last_seen: Option<String>,
}

impl From<&KnownDevice> for KnownDeviceDto {
    fn from(d: &KnownDevice) -> Self {
        Self {
            device_id: d.device_id.as_str().to_owned(),
            display_name: d.display_name.clone(),
            public_key_b64: b64_encode(&d.public_key),
            first_seen: ts_to_string(d.first_seen),
            last_seen: d.last_seen.map(ts_to_string),
        }
    }
}

impl KnownDeviceDto {
    pub fn into_domain(self) -> Result<KnownDevice, CommandError> {
        Ok(KnownDevice {
            device_id: DeviceId::from_raw(self.device_id),
            display_name: self.display_name,
            public_key: b64_decode_fixed::<32>(&self.public_key_b64)?,
            first_seen: ts_from_string(&self.first_seen)?,
            last_seen: self.last_seen.as_deref().map(ts_from_string).transpose()?,
        })
    }
}

// ---- ExtensionSession --------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSessionDto {
    pub session_id: String,
    pub browser: String,
    #[serde(default)]
    pub profile_name: Option<String>,
    /// Base64 of the 32-byte session key. Not a password — an IPC shared
    /// secret. Per the app-db spec, exposure lets an attacker pretend to
    /// be the extension but the vault must still be unlocked for autofill
    /// to work.
    pub session_key_b64: String,
    pub created_at: String,
    #[serde(default)]
    pub last_active_at: Option<String>,
}

impl From<&ExtensionSession> for ExtensionSessionDto {
    fn from(s: &ExtensionSession) -> Self {
        Self {
            session_id: s.session_id.as_str().to_owned(),
            browser: s.browser.clone(),
            profile_name: s.profile_name.clone(),
            session_key_b64: b64_encode(&s.session_key),
            created_at: ts_to_string(s.created_at),
            last_active_at: s.last_active_at.map(ts_to_string),
        }
    }
}

impl ExtensionSessionDto {
    pub fn into_domain(self) -> Result<ExtensionSession, CommandError> {
        Ok(ExtensionSession {
            session_id: SessionId::from_raw(self.session_id),
            browser: self.browser,
            profile_name: self.profile_name,
            session_key: b64_decode_fixed::<32>(&self.session_key_b64)?,
            created_at: ts_from_string(&self.created_at)?,
            last_active_at: self
                .last_active_at
                .as_deref()
                .map(ts_from_string)
                .transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use chrono::Utc;

    #[test]
    fn theme_round_trip_preserves_fields() {
        let now = Utc::now();
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
            created_at: now,
            updated_at: now,
        };
        let dto = ThemeDto::from(&t);
        let back = dto.into_domain().unwrap();
        assert_eq!(back.id, t.id);
        assert_eq!(back.name, t.name);
        assert_eq!(back.is_built_in, t.is_built_in);
        assert_eq!(back.created_at.timestamp_millis(), t.created_at.timestamp_millis());
    }

    #[test]
    fn known_device_round_trips_public_key() {
        let pk: [u8; 32] = std::array::from_fn(|i| u8::try_from(i & 0xff).unwrap());
        let d = KnownDevice {
            device_id: DeviceId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            display_name: "laptop".into(),
            public_key: pk,
            first_seen: Utc::now(),
            last_seen: None,
        };
        let dto = KnownDeviceDto::from(&d);
        let back = dto.into_domain().unwrap();
        assert_eq!(back.public_key, pk);
    }

    #[test]
    fn extension_session_rejects_wrong_key_length() {
        let dto = ExtensionSessionDto {
            session_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
            browser: "firefox".into(),
            profile_name: None,
            session_key_b64: crate::dto::common::b64_encode(&[0u8; 16]),
            created_at: "2026-01-01T00:00:00.000Z".into(),
            last_active_at: None,
        };
        let err = dto.into_domain().unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }
}
