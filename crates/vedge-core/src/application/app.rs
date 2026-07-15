pub mod ports;
pub mod use_cases;

pub use ports::{
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository, ThemeRepository,
    VaultRegistry,
};
pub use use_cases::{
    ACTIVE_THEME_SETTING_KEY, DEFAULT_THEME_ID, RegisteredVaultStatus,
    list_registered_vaults_with_status, resolve_active_theme, set_active_theme,
};
