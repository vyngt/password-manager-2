//! Entry CRUD + `copy_field` + `move_entry`.
//!
//! See `commands/vault.rs` for the rationale behind the `#![allow]` below
//! (macro expansion of `#[tauri::command]` trips `unreachable` /
//! `let_underscore_must_use`; holding the session mutex across the full
//! command body trips `significant_drop_tightening`).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]
//!
//! Every command here acquires the session's async mutex, converts the
//! DTO input into the domain payload, and delegates to the matching
//! `vedge_core` use case. Secrets cross this boundary as plain `String`s
//! inside the DTO, get wrapped in `SecretString` during conversion, and
//! drop with the DTO as soon as the use case returns.

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::{EntryId, VaultId};
use vedge_core::{
    CopyFieldInput, CreateEntryInput, UpdateEntryInput, copy_field as copy_field_core,
    create_entry as create_entry_core, hard_delete_entry as hard_delete_entry_core,
    move_entry as move_entry_core, restore_entry as restore_entry_core,
    soft_delete_entry as soft_delete_entry_core, update_entry as update_entry_core,
};

use crate::dto::entry::{PayloadDto, entry_id_from_str, payload_from_dto};
use crate::dto::misc::{FieldSelectorDto, field_selector_from_dto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn create_entry(
    vault_path: String,
    payload: PayloadDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let domain_payload = payload_from_dto(payload)?;
    let out = create_entry_core(
        &mut guard,
        CreateEntryInput {
            payload: domain_payload,
        },
    )
    .await?;
    Ok(out.entry_id.into_string())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn update_entry(
    vault_path: String,
    entry_id: String,
    payload: PayloadDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let domain_payload = payload_from_dto(payload)?;
    update_entry_core(
        &mut guard,
        UpdateEntryInput {
            entry_id: id,
            payload: domain_payload,
        },
    )
    .await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn soft_delete_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    soft_delete_entry_core(&mut guard, &id).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn restore_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    restore_entry_core(&mut guard, &id).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn hard_delete_entry(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    hard_delete_entry_core(&mut guard, &id).await?;
    Ok(())
}

/// `clear_after_secs = None` means "use the use-case default" (30 s). The
/// use case takes a concrete `u32`, so we substitute 30 here.
#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn copy_field(
    vault_path: String,
    entry_id: String,
    field: FieldSelectorDto,
    clear_after_secs: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    copy_field_core(
        &mut guard,
        CopyFieldInput {
            entry_id: entry_id_from_str(&entry_id),
            field: field_selector_from_dto(field),
            clear_after_secs: clear_after_secs.unwrap_or(30),
        },
    )
    .await?;
    Ok(())
}

/// `folder_id = None` moves the entry to the root. Matches the use case's
/// `Option<&EntryId>` signature.
#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn move_entry(
    vault_path: String,
    entry_id: String,
    folder_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let folder: Option<EntryId> = folder_id.as_deref().map(entry_id_from_str);
    move_entry_core(&mut guard, &id, folder.as_ref()).await?;
    Ok(())
}
