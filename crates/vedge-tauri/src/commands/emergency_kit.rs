//! Emergency Kit export commands.
//!
//! - [`export_emergency_kit`] — structured content (JSON shape for a UI
//!   viewer or alternative renderer).
//! - [`emergency_kit_pdf`] — the same content rendered to PDF bytes.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;
use std::sync::Arc;

use tracing::instrument;

use vedge_core::application::vault::ports::KeychainProvider;
use vedge_core::domain::shared::VaultId;
use vedge_core::{ExportEmergencyKitInput, export_emergency_kit as core_export};

use crate::dto::emergency_kit::EmergencyKitDto;
use crate::error::CommandError;
use crate::pdf;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

async fn resolve_display_name(state: &AppState, vault_path: &str) -> Option<String> {
    // `list_recent_vaults` is cheap; the recent-vault table is tiny.
    let recents = state.recent_vaults.list().await.ok()?;
    recents
        .into_iter()
        .find(|r| r.path.to_string_lossy() == vault_path)
        .map(|r| r.display_name)
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn export_emergency_kit(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<EmergencyKitDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let display_name = resolve_display_name(&state, &vault_path).await;

    let content = core_export(
        &guard,
        Arc::clone(&state.keychain) as Arc<dyn KeychainProvider>,
        ExportEmergencyKitInput {
            vault_display_name: display_name,
        },
    )
    .await?;

    Ok(content.into())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn emergency_kit_pdf(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<u8>, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let display_name = resolve_display_name(&state, &vault_path).await;

    let content = core_export(
        &guard,
        Arc::clone(&state.keychain) as Arc<dyn KeychainProvider>,
        ExportEmergencyKitInput {
            vault_display_name: display_name,
        },
    )
    .await?;

    pdf::emergency_kit::render(&content)
}
