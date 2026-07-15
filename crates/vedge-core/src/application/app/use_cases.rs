//! App-level use cases. One module per repository family.
//!
//! Every `#[tauri::command]` in the shell routes through one of these —
//! even the one-line pass-throughs. That uniformity is the whole point
//! of the layer: future instrumentation, validation, and coordination
//! attach here without touching `vedge-tauri`.

pub mod app_setting_ops;
pub mod extension_session_ops;
pub mod known_device_ops;
pub mod theme_ops;
pub mod vault_registry_ops;

pub use app_setting_ops::{
    delete_app_setting, get_app_setting, list_app_settings, set_app_setting,
};
pub use extension_session_ops::{
    delete_extension_session, list_extension_sessions, touch_extension_session_last_active,
    upsert_extension_session,
};
pub use known_device_ops::{
    delete_known_device, list_known_devices, touch_known_device_last_seen, upsert_known_device,
};
pub use theme_ops::{
    ACTIVE_THEME_SETTING_KEY, CreateCustomThemeInput, DEFAULT_THEME_ID, UpdateCustomThemeInput,
    create_custom_theme, delete_custom_theme, duplicate_theme, get_theme, list_themes,
    resolve_active_theme, set_active_theme, update_custom_theme, validate_hex_color,
};
pub use vault_registry_ops::record_vault_uuid;
pub use vault_registry_ops::{
    RegisterVaultInput, RegisteredVaultStatus, SQLITE_MAGIC, deregister_vault,
    healed_registry_path, list_registered_vaults, list_registered_vaults_with_status,
    register_vault, remove_stale_recents, rename_registered_vault, repoint_registered_vault,
    touch_on_unlock, touch_registered_vault,
};
