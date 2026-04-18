//! `app_settings` + `themes` app.db commands.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (no session
//! mutex here, but the macro-generated patterns still trip the first
//! two lints).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use serde_json::Value;
use tracing::instrument;

use vedge_core::domain::shared::{now, ThemeId};

use crate::dto::settings::{AppSettingDto, ThemeDto};
use crate::error::CommandError;
use crate::state::AppState;

// ---- app_settings ------------------------------------------------------------

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn get_app_setting(
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<Option<AppSettingDto>, CommandError> {
    let row = state.app_settings.get(&key).await?;
    Ok(row.as_ref().map(AppSettingDto::from))
}

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn set_app_setting(
    key: String,
    value: Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    state.app_settings.set(&key, value, now()).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(key = %key))]
pub async fn delete_app_setting(
    key: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    state.app_settings.delete(&key).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(prefix = %prefix))]
pub async fn list_app_settings(
    prefix: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppSettingDto>, CommandError> {
    let rows = state.app_settings.list_prefix(&prefix).await?;
    Ok(rows.iter().map(AppSettingDto::from).collect())
}

// ---- themes ------------------------------------------------------------------

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_themes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ThemeDto>, CommandError> {
    let rows = state.themes.list().await?;
    Ok(rows.iter().map(ThemeDto::from).collect())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn get_theme(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ThemeDto, CommandError> {
    let tid = ThemeId::from_raw(id);
    let row = state.themes.get(&tid).await?;
    Ok(ThemeDto::from(&row))
}

#[tauri::command]
#[instrument(skip_all, fields(id = %theme.id))]
pub async fn upsert_theme(
    theme: ThemeDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = theme.into_domain()?;
    state.themes.upsert(&domain).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn delete_theme(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let tid = ThemeId::from_raw(id);
    state.themes.delete(&tid).await?;
    Ok(())
}
