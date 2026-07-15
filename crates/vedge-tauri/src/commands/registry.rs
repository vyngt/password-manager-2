//! `vault_registry` app.db commands. Every command routes through a
//! `vedge-core` use case — no direct `state.vault_registry.*` calls.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion
//! of `#[tauri::command(rename_all = "snake_case")]`).

#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::infrastructure::backup::target::read_target_state;
use vedge_core::{
    RegisterVaultInput, RegisteredVaultStatus, deregister_vault as deregister_vault_core,
    healed_registry_path, list_registered_vaults as list_registered_vaults_core,
    list_registered_vaults_with_status as list_registered_vaults_with_status_core,
    record_vault_uuid as record_vault_uuid_core, register_vault as register_vault_core,
    remove_stale_recents as remove_stale_recents_core,
    rename_registered_vault as rename_registered_vault_core,
    repoint_registered_vault as repoint_registered_vault_core,
    touch_on_unlock as touch_on_unlock_core, touch_registered_vault as touch_registered_vault_core,
};

use crate::commands::probe_vault_uuid;
use crate::dto::settings::{
    RegisteredVaultDto, RegisteredVaultStatusDto, registered_vault_status_to_dto,
    registered_vault_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_registered_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RegisteredVaultDto>, CommandError> {
    let vaults = list_registered_vaults_core(&*state.vault_registry).await?;
    Ok(vaults.iter().map(registered_vault_to_dto).collect())
}

/// Onboarding variant: list + per-row filesystem existence check.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn list_registered_vaults_with_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RegisteredVaultStatusDto>, CommandError> {
    let rows = list_registered_vaults_with_status_core(&*state.vault_registry).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        // B3 self-heal (5.2.3): a row whose path is gone but whose `<stem>.vedge/` sibling holds
        // a readable `vault.vdb` (a legacy `.vdb` converted, or a vault moved in place) is
        // re-pointed. The vault probe is read-only; only the app.db row is written.
        if !r.exists
            && let Some(healed) = healed_registry_path(&r.vault.path)
        {
            // Best-effort — a heal failure must not break the whole list.
            let _ =
                repoint_registered_vault_core(&*state.vault_registry, &r.vault.id, healed.clone())
                    .await;
            let openable = read_target_state(&healed).await.identity().is_some();
            let mut vault = r.vault.clone();
            vault.path = healed;
            let healed_status = RegisteredVaultStatus {
                vault,
                exists: true,
            };
            out.push(registered_vault_status_to_dto(&healed_status, openable));
            continue;
        }
        // H0: a present-but-unopenable vault (its `vault.vdb` won't read as a DB) is CORRUPT,
        // so `openable` is false and the launch screen offers Restore. Eager per-row; fine for
        // a handful of recents.
        let openable = r.exists && read_target_state(&r.vault.path).await.identity().is_some();
        out.push(registered_vault_status_to_dto(r, openable));
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
pub async fn register_vault(
    dto: RegisteredVaultDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let path = PathBuf::from(dto.path);
    let vault_uuid = probe_vault_uuid(&path).await;
    register_vault_core(
        &*state.vault_registry,
        RegisterVaultInput {
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
pub async fn deregister_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    deregister_vault_core(&*state.vault_registry, &id).await?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_registered_vault(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_registered_vault_core(&*state.vault_registry, &id).await?;
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
pub async fn touch_registered_vault_on_unlock(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    touch_on_unlock_core(&*state.vault_registry, &id).await?;

    if let Ok(row) = state.vault_registry.get(&id).await
        && row.vault_uuid.is_none()
        && let Some(uuid) = probe_vault_uuid(&row.path).await
        && let Err(e) = record_vault_uuid_core(&*state.vault_registry, &id, &uuid).await
    {
        tracing::warn!(error = %e, "unlocked, but the recents row's vault_uuid could not be backfilled");
    }
    Ok(())
}

/// Rename a recent vault's display name (inline edit in the vault picker).
/// Rejects an empty/whitespace name.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(id = %id))]
pub async fn rename_registered_vault(
    id: String,
    display_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    rename_registered_vault_core(&*state.vault_registry, &id, &display_name).await?;
    Ok(())
}

/// Batch-delete entries whose paths no longer exist on disk. Returns the
/// count for a "cleaned up N" toast.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all)]
pub async fn remove_stale_registered_vaults(
    state: tauri::State<'_, AppState>,
) -> Result<u64, CommandError> {
    Ok(remove_stale_recents_core(&*state.vault_registry).await?)
}
