//! `recent_vaults` app.db commands.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (no session
//! mutex here, but the macro-generated patterns still trip the first
//! two lints).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use tracing::instrument;
use vedge_core::domain::shared::now;

use crate::dto::settings::RecentVaultDto;
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_recent_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RecentVaultDto>, CommandError> {
    let vaults = state.recent_vaults.list().await?;
    Ok(vaults.iter().map(RecentVaultDto::from).collect())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %dto.id))]
pub async fn add_recent_vault(
    dto: RecentVaultDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let domain = dto.into_domain()?;
    state.recent_vaults.upsert(&domain).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn remove_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    state.recent_vaults.delete(&id).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    state.recent_vaults.touch_last_opened(&id, now()).await?;
    Ok(())
}
