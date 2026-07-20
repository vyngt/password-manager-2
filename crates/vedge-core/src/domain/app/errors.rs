use thiserror::Error;

use crate::domain::shared::{DeviceId, SessionId, StorageError, ThemeId};

#[derive(Debug, Error)]
pub enum AppDbError {
    #[error("recent vault not found: {0}")]
    RecentVaultNotFound(String),

    #[error("setting not found: {0}")]
    SettingNotFound(String),

    #[error("theme not found: {0}")]
    ThemeNotFound(ThemeId),

    #[error("built-in theme cannot be modified or deleted")]
    BuiltInThemeImmutable,

    #[error("device not found: {0}")]
    DeviceNotFound(DeviceId),

    #[error("extension session not found: {0}")]
    ExtensionSessionNotFound(SessionId),

    #[error("invalid setting value for key {key}: {reason}")]
    InvalidSettingValue { key: String, reason: String },

    #[error("storage failure")]
    Storage(#[source] StorageError),
}

impl From<StorageError> for AppDbError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}
