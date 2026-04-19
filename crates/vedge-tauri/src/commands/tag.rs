//! Tag CRUD commands. Tags store a normalized form under the KEK; the
//! use cases handle normalization so the shell passes raw user input
//! straight through.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::{
    create_tag as create_tag_core, delete_tag as delete_tag_core, rename_tag as rename_tag_core,
};

use crate::dto::entry::tag_id_from_str;
use crate::dto::tag::{CreateTagDto, RenameTagDto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn create_tag(
    vault_path: String,
    input: CreateTagDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = create_tag_core(&mut guard, &input.name, input.color).await?;
    Ok(id.into_string())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, tag_id = %input.tag_id))]
pub async fn rename_tag(
    vault_path: String,
    input: RenameTagDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let tid = tag_id_from_str(&input.tag_id);
    rename_tag_core(&mut guard, &tid, &input.new_name).await?;
    Ok(())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, tag_id = %tag_id))]
pub async fn delete_tag(
    vault_path: String,
    tag_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let tid = tag_id_from_str(&tag_id);
    delete_tag_core(&mut guard, &tid).await?;
    Ok(())
}
