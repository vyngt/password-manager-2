//! App.db wire types: recent vaults, settings, themes, known devices,
//! extension sessions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---- RecentVault -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentVaultDto {
    pub id: String,
    /// Path as a string — platform-native separator.
    pub path: String,
    pub display_name: String,
    #[serde(default)]
    pub last_opened: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentVaultStatusDto {
    #[serde(flatten)]
    pub vault: RecentVaultDto,
    pub exists: bool,
}

// ---- AppSetting --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingDto {
    pub key: String,
    /// Stored as raw JSON — any shape the caller wants.
    pub value: Value,
    pub updated_at: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCustomThemeInputDto {
    pub name: String,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    #[serde(default)]
    pub danger_base: Option<String>,
    #[serde(default)]
    pub warning_base: Option<String>,
    #[serde(default)]
    pub success_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCustomThemeInputDto {
    pub id: String,
    pub name: String,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    #[serde(default)]
    pub danger_base: Option<String>,
    #[serde(default)]
    pub warning_base: Option<String>,
    #[serde(default)]
    pub success_base: Option<String>,
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

// ---- ExtensionSession --------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSessionDto {
    pub session_id: String,
    pub browser: String,
    #[serde(default)]
    pub profile_name: Option<String>,
    /// Base64 of the 32-byte session key.
    pub session_key_b64: String,
    pub created_at: String,
    #[serde(default)]
    pub last_active_at: Option<String>,
}
