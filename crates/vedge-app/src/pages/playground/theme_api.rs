//! DTO ↔ `ThemeConfig` converters for the theme playground demo.
//!
//! Pure-data helpers: one file, three functions, zero IPC.
//!
//! The field-name delta between the wire shape and the editor's live
//! state is the only interesting thing here — `root_*` prefixes drop on
//! the way into `ThemeConfig`, and the optional status-color fields keep
//! the same `Option<String>` semantics.

use vedge_ipc::{CreateCustomThemeInputDto, ThemeDto, UpdateCustomThemeInputDto};
use vedge_ui::theme::ThemeConfig;

/// Extract the six editable colors out of a persisted theme.
#[must_use]
pub fn theme_config_from_dto(dto: &ThemeDto) -> ThemeConfig {
    ThemeConfig {
        background: dto.root_background.clone(),
        foreground: dto.root_foreground.clone(),
        primary: dto.root_primary.clone(),
        danger: dto.danger_base.clone(),
        warning: dto.warning_base.clone(),
        success: dto.success_base.clone(),
    }
}

/// Build a create-input from the editor's current state + a user-chosen name.
#[must_use]
pub fn create_input_from(name: String, config: &ThemeConfig) -> CreateCustomThemeInputDto {
    CreateCustomThemeInputDto {
        name,
        root_background: config.background.clone(),
        root_foreground: config.foreground.clone(),
        root_primary: config.primary.clone(),
        danger_base: config.danger.clone(),
        warning_base: config.warning.clone(),
        success_base: config.success.clone(),
    }
}

/// Build an update-input from the editor's current state for an existing
/// custom theme.
#[must_use]
pub fn update_input_from(
    id: String,
    name: String,
    config: &ThemeConfig,
) -> UpdateCustomThemeInputDto {
    UpdateCustomThemeInputDto {
        id,
        name,
        root_background: config.background.clone(),
        root_foreground: config.foreground.clone(),
        root_primary: config.primary.clone(),
        danger_base: config.danger.clone(),
        warning_base: config.warning.clone(),
        success_base: config.success.clone(),
    }
}
