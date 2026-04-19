//! `app_settings` + `themes` app.db commands. Every command routes
//! through a `vedge-core` use case.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use serde_json::Value;
use tracing::instrument;

use vedge_core::domain::shared::ThemeId;
use vedge_core::{
    create_custom_theme as create_custom_theme_core,
    delete_app_setting as delete_app_setting_core,
    delete_custom_theme as delete_custom_theme_core, duplicate_theme as duplicate_theme_core,
    get_app_setting as get_app_setting_core, get_theme as get_theme_core,
    list_app_settings as list_app_settings_core, list_themes as list_themes_core,
    resolve_active_theme as resolve_active_theme_core,
    set_active_theme as set_active_theme_core, set_app_setting as set_app_setting_core,
    update_custom_theme as update_custom_theme_core, CreateCustomThemeInput,
    UpdateCustomThemeInput,
};

use crate::dto::settings::{
    AppSettingDto, CreateCustomThemeInputDto, ThemeDto, UpdateCustomThemeInputDto,
};
use crate::error::CommandError;
use crate::state::AppState;

// ---- app_settings ------------------------------------------------------------

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn get_app_setting(
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<AppSettingDto>, CommandError> {
    let row = get_app_setting_core(&*state.app_settings, &key).await?;
    Ok(row.as_ref().map(AppSettingDto::from))
}

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn set_app_setting(
    key: String,
    value: Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    set_app_setting_core(&*state.app_settings, &key, value).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn delete_app_setting(
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    delete_app_setting_core(&*state.app_settings, &key).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(prefix = %prefix))]
pub async fn list_app_settings(
    prefix: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppSettingDto>, CommandError> {
    let rows = list_app_settings_core(&*state.app_settings, &prefix).await?;
    Ok(rows.iter().map(AppSettingDto::from).collect())
}

// ---- themes ------------------------------------------------------------------

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_themes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ThemeDto>, CommandError> {
    let rows = list_themes_core(&*state.themes).await?;
    Ok(rows.iter().map(ThemeDto::from).collect())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn get_theme(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ThemeDto, CommandError> {
    let tid = ThemeId::from_raw(id);
    let row = get_theme_core(&*state.themes, &tid).await?;
    Ok(ThemeDto::from(&row))
}

/// Resolve the active theme, falling back to the built-in default if the
/// setting is unset or points at a missing row. Called on app startup.
#[tauri::command]
#[instrument(skip_all)]
pub async fn get_active_theme(
    state: tauri::State<'_, AppState>,
) -> Result<ThemeDto, CommandError> {
    let theme =
        resolve_active_theme_core(&*state.themes, &*state.app_settings).await?;
    Ok(ThemeDto::from(&theme))
}

/// Set the active theme. Validates the target exists before writing.
#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn set_active_theme(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let tid = ThemeId::from_raw(id);
    set_active_theme_core(&*state.themes, &*state.app_settings, &tid).await?;
    Ok(())
}

/// Create a user-defined theme. Validates colors + name, forces
/// `is_built_in = false`.
#[tauri::command]
#[instrument(skip_all, fields(name = %input.name))]
pub async fn create_custom_theme(
    input: CreateCustomThemeInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let id = create_custom_theme_core(
        &*state.themes,
        CreateCustomThemeInput {
            name: input.name,
            root_background: input.root_background,
            root_foreground: input.root_foreground,
            root_primary: input.root_primary,
            danger_base: input.danger_base,
            warning_base: input.warning_base,
            success_base: input.success_base,
        },
    )
    .await?;
    Ok(id.into_string())
}

/// Mutate an existing custom theme. Built-ins are rejected.
#[tauri::command]
#[instrument(skip_all, fields(id = %input.id))]
pub async fn update_custom_theme(
    input: UpdateCustomThemeInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    update_custom_theme_core(
        &*state.themes,
        UpdateCustomThemeInput {
            id: ThemeId::from_raw(input.id),
            name: input.name,
            root_background: input.root_background,
            root_foreground: input.root_foreground,
            root_primary: input.root_primary,
            danger_base: input.danger_base,
            warning_base: input.warning_base,
            success_base: input.success_base,
        },
    )
    .await?;
    Ok(())
}

/// Clone a theme. `new_name = None` → `"{source} (copy)"`.
#[tauri::command]
#[instrument(skip_all, fields(source_id = %source_id))]
pub async fn duplicate_theme(
    source_id: String,
    new_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let src = ThemeId::from_raw(source_id);
    let id = duplicate_theme_core(&*state.themes, &src, new_name).await?;
    Ok(id.into_string())
}

/// Delete a custom theme. Rejects built-ins. If the theme was the active
/// one, clears the active-theme setting first.
#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn delete_custom_theme(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let tid = ThemeId::from_raw(id);
    delete_custom_theme_core(&*state.themes, &*state.app_settings, &tid).await?;
    Ok(())
}
