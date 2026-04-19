//! `recent_vaults` app.db commands. Every command routes through a
//! `vedge-core` use case — no direct `state.recent_vaults.*` calls.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion
//! of `#[tauri::command]`).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::{
    add_recent_vault as add_recent_vault_core, list_recent_vaults as list_recent_vaults_core,
    list_recent_vaults_with_status as list_recent_vaults_with_status_core,
    remove_recent_vault as remove_recent_vault_core, remove_stale_recents as remove_stale_recents_core,
    touch_on_unlock as touch_on_unlock_core, touch_recent_vault as touch_recent_vault_core,
    AddRecentVaultInput,
};

use crate::dto::settings::{RecentVaultDto, RecentVaultStatusDto};
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command]
#[instrument(skip_all)]
pub async fn list_recent_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RecentVaultDto>, CommandError> {
    let vaults = list_recent_vaults_core(&*state.recent_vaults).await?;
    Ok(vaults.iter().map(RecentVaultDto::from).collect())
}

/// Onboarding variant: list + per-row filesystem existence check.
#[tauri::command]
#[instrument(skip_all)]
pub async fn list_recent_vaults_with_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RecentVaultStatusDto>, CommandError> {
    let rows = list_recent_vaults_with_status_core(&*state.recent_vaults).await?;
    Ok(rows.iter().map(RecentVaultStatusDto::from).collect())
}

/// Strict add: validates the file exists and has a `SQLite` header
/// before persisting to the recents list.
#[tauri::command]
#[instrument(skip_all, fields(id = %dto.id))]
pub async fn add_recent_vault(
    dto: RecentVaultDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    add_recent_vault_core(
        &*state.recent_vaults,
        AddRecentVaultInput {
            id: dto.id,
            path: PathBuf::from(dto.path),
            display_name: dto.display_name,
            sort_order: dto.sort_order,
        },
    )
    .await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn remove_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    remove_recent_vault_core(&*state.recent_vaults, &id).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_recent_vault_core(&*state.recent_vaults, &id).await?;
    Ok(())
}

/// Bump `last_opened` + move to top. Called by the shell after a
/// successful unlock.
#[tauri::command]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault_on_unlock(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_on_unlock_core(&*state.recent_vaults, &id).await?;
    Ok(())
}

/// Batch-delete entries whose paths no longer exist on disk. Returns the
/// count for a "cleaned up N" toast.
#[tauri::command]
#[instrument(skip_all)]
pub async fn remove_stale_recent_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<u64, CommandError> {
    Ok(remove_stale_recents_core(&*state.recent_vaults).await?)
}
