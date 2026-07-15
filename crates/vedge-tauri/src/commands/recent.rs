//! `recent_vaults` app.db commands. Every command routes through a
//! `vedge-core` use case — no direct `state.recent_vaults.*` calls.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion
//! of `#[tauri::command(rename_all = "snake_case")]`).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::infrastructure::backup::target::{TargetState, read_target_state};
use vedge_core::{
    AddRecentVaultInput, add_recent_vault as add_recent_vault_core,
    list_recent_vaults as list_recent_vaults_core,
    list_recent_vaults_with_status as list_recent_vaults_with_status_core,
    record_vault_uuid as record_vault_uuid_core, remove_recent_vault as remove_recent_vault_core,
    remove_stale_recents as remove_stale_recents_core,
    rename_recent_vault as rename_recent_vault_core, touch_on_unlock as touch_on_unlock_core,
    touch_recent_vault as touch_recent_vault_core,
};

use crate::dto::settings::{
    RecentVaultDto, RecentVaultStatusDto, recent_vault_status_to_dto, recent_vault_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Read a vault home's plaintext `vault_uuid` without unlocking it.
///
/// 🔴 Lives in the SHELL, not in the app-layer use case. `vault_uuid` is *vault* infrastructure
/// and `recent_vaults` is *app* infrastructure; having the app use case reach across to open a
/// vault would invert the dependency. The shell composes the two — the same layering the
/// `openable` probe below already respects.
async fn probe_vault_uuid(home: &std::path::Path) -> Option<String> {
    match read_target_state(home).await {
        TargetState::Readable(id) => id.vault_uuid,
        TargetState::Missing | TargetState::Unreadable => None,
    }
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_recent_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RecentVaultDto>, CommandError> {
    let vaults = list_recent_vaults_core(&*state.recent_vaults).await?;
    Ok(vaults.iter().map(recent_vault_to_dto).collect())
}

/// Onboarding variant: list + per-row filesystem existence check.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_recent_vaults_with_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RecentVaultStatusDto>, CommandError> {
    let rows = list_recent_vaults_with_status_core(&*state.recent_vaults).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        // H0: a present-but-unopenable vault (its `vault.vdb` won't read as a DB) is CORRUPT,
        // so `openable` is false and the launch screen offers Restore. Eager per-row; fine for
        // a handful of recents.
        let openable = r.exists && read_target_state(&r.vault.path).await.identity().is_some();
        out.push(recent_vault_status_to_dto(r, openable));
    }
    Ok(out)
}

/// Strict add: validates the vault home exists and its `vault.vdb` has a `SQLite` header
/// before persisting to the recents list.
///
/// Also captures the vault's plaintext `vault_uuid` (slice 5.2.2), which is what lets ②'s
/// duplicate check answer *"is this identity already on this machine?"* — the check that keeps
/// a copy from sharing a rollback baseline with its original and making it cry wolf forever.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %dto.id))]
pub async fn add_recent_vault(
    dto: RecentVaultDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let path = PathBuf::from(dto.path);
    let vault_uuid = probe_vault_uuid(&path).await;
    add_recent_vault_core(
        &*state.recent_vaults,
        AddRecentVaultInput {
            id: dto.id,
            path,
            display_name: dto.display_name,
            sort_order: dto.sort_order,
            vault_uuid,
        },
    )
    .await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn remove_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    remove_recent_vault_core(&*state.recent_vaults, &id).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_recent_vault_core(&*state.recent_vaults, &id).await?;
    Ok(())
}

/// Bump `last_opened` — recency floats the row to the top. Called by the
/// shell after a successful unlock.
///
/// Also **backfills `vault_uuid`** (slice 5.2.2). Rows written before the uuid was captured
/// would otherwise stay unknown forever, leaving ②'s duplicate check with an incomplete picture
/// of what this machine holds. An unlock is the natural moment: we have just proved the vault is
/// readable. Best-effort — a failure here must never fail the unlock that triggered it.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault_on_unlock(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_on_unlock_core(&*state.recent_vaults, &id).await?;

    if let Ok(row) = state.recent_vaults.get(&id).await
        && row.vault_uuid.is_none()
        && let Some(uuid) = probe_vault_uuid(&row.path).await
        && let Err(e) = record_vault_uuid_core(&*state.recent_vaults, &id, &uuid).await
    {
        tracing::warn!(error = %e, "unlocked, but the recents row's vault_uuid could not be backfilled");
    }
    Ok(())
}

/// Rename a recent vault's display name (inline edit in the vault picker).
/// Rejects an empty/whitespace name.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn rename_recent_vault(
    id: String,
    display_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    rename_recent_vault_core(&*state.recent_vaults, &id, &display_name).await?;
    Ok(())
}

/// Batch-delete entries whose paths no longer exist on disk. Returns the
/// count for a "cleaned up N" toast.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn remove_stale_recent_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<u64, CommandError> {
    Ok(remove_stale_recents_core(&*state.recent_vaults).await?)
}
