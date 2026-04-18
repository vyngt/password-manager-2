use async_trait::async_trait;
use serde_json::Value;

use crate::domain::app::entities::{
    AppSetting, ExtensionSession, KnownDevice, RecentVault, Theme,
};
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{DeviceId, SessionId, ThemeId, Timestamp};

#[async_trait]
pub trait RecentVaultRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<RecentVault>, AppDbError>;
    async fn get(&self, id: &str) -> Result<RecentVault, AppDbError>;
    async fn upsert(&self, vault: &RecentVault) -> Result<(), AppDbError>;
    async fn delete(&self, id: &str) -> Result<(), AppDbError>;
    async fn touch_last_opened(&self, id: &str, when: Timestamp) -> Result<(), AppDbError>;
}

#[async_trait]
pub trait AppSettingRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<AppSetting>, AppDbError>;
    async fn set(&self, key: &str, value: Value, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, key: &str) -> Result<(), AppDbError>;
    async fn list_prefix(&self, prefix: &str) -> Result<Vec<AppSetting>, AppDbError>;
}

#[async_trait]
pub trait ThemeRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Theme>, AppDbError>;
    async fn get(&self, id: &ThemeId) -> Result<Theme, AppDbError>;
    async fn upsert(&self, theme: &Theme) -> Result<(), AppDbError>;
    async fn delete(&self, id: &ThemeId) -> Result<(), AppDbError>;
}

#[async_trait]
pub trait KnownDeviceRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<KnownDevice>, AppDbError>;
    async fn upsert(&self, device: &KnownDevice) -> Result<(), AppDbError>;
    async fn touch_last_seen(&self, id: &DeviceId, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, id: &DeviceId) -> Result<(), AppDbError>;
}

#[async_trait]
pub trait ExtensionSessionRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ExtensionSession>, AppDbError>;
    async fn upsert(&self, session: &ExtensionSession) -> Result<(), AppDbError>;
    async fn touch_last_active(&self, id: &SessionId, when: Timestamp) -> Result<(), AppDbError>;
    async fn delete(&self, id: &SessionId) -> Result<(), AppDbError>;
}
