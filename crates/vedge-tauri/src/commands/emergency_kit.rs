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

use crate::dto::emergency_kit::{EmergencyKitDto, emergency_kit_to_dto};
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

#[tauri::command(rename_all = "snake_case")]
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

    Ok(emergency_kit_to_dto(content))
}

/// Render the Emergency Kit PDF bytes for an unlocked vault. Shared by the
/// bytes-returning command and the write-to-disk command.
async fn render_kit_pdf(state: &AppState, vault_path: &str) -> Result<Vec<u8>, CommandError> {
    let vault_id = vault_id_from_string(vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let display_name = resolve_display_name(state, vault_path).await;

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

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn emergency_kit_pdf(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<u8>, CommandError> {
    render_kit_pdf(&state, &vault_path).await
}

/// Render the Emergency Kit PDF and write it to `dest_path`.
///
/// `dest_path` is a path the user chose via the native save dialog. Uses
/// `std::fs` directly — the fs plugin is scope-gated and would reject an
/// arbitrary dialog-picked path.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn write_emergency_kit_pdf(
    vault_path: String,
    dest_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let bytes = render_kit_pdf(&state, &vault_path).await?;
    std::fs::write(&dest_path, &bytes)
        .map_err(|e| CommandError::Storage(format!("write emergency kit pdf: {e}")))
}
