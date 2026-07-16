//! Import commands (slice 5.3b) — begin a staged import, commit the picked rows,
//! or cancel it.
//!
//! Mirrors the export command's session-guard + `Zeroizing`-passphrase discipline.
//! The preview crosses **derivatives only** (Decision ⑦); the passphrase (for an
//! encrypted source) and the pasted CSV text (Decision ⑧) travel *in* and are
//! wrapped/consumed here — nothing secret comes back. The backend reads a picked
//! file with `std::fs`; its bytes never reach WASM.

// The `#[tauri::command]` expansion emits an `unreachable!` arm and a non-binding
// `let _ = …` on a `#[must_use]` value; allowed in every command module for the
// same reason.
#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;
use zeroize::Zeroizing;

use vedge_core::domain::shared::VaultId;
use vedge_core::{
    ImportSource, begin_import as begin_import_core, cancel_import as cancel_import_core,
    commit_import as commit_import_core,
};

use crate::dto::import::{
    ImportActionDto, ImportPreviewRow, ImportReportDto, action_from_dto, preview_row_to_dto,
    report_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Turn the command args into an [`ImportSource`], reading a picked file with
/// `std::fs` when needed. Pasted text (Decision ⑧, no file) wins over a file path.
fn read_source(
    src_path: Option<String>,
    paste_text: Option<String>,
    encrypted: bool,
    passphrase: Option<String>,
) -> Result<ImportSource, CommandError> {
    if encrypted {
        let path = src_path
            .ok_or_else(|| CommandError::Invalid("an encrypted import needs a file".into()))?;
        let data = std::fs::read(&path)
            .map_err(|e| CommandError::Storage(format!("read import file: {e}")))?;
        let pass = passphrase
            .filter(|p| !p.is_empty())
            .ok_or_else(|| CommandError::Invalid("an import passphrase is required".into()))?;
        return Ok(ImportSource::Encrypted {
            data,
            passphrase: Zeroizing::new(pass),
        });
    }
    if let Some(text) = paste_text.filter(|t| !t.is_empty()) {
        return Ok(ImportSource::Csv { text });
    }
    let path = src_path
        .ok_or_else(|| CommandError::Invalid("a CSV import needs a file or text".into()))?;
    let text = std::fs::read_to_string(&path)
        .map_err(|e| CommandError::Storage(format!("read CSV file: {e}")))?;
    Ok(ImportSource::Csv { text })
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, encrypted))]
pub async fn begin_import(
    vault_path: String,
    src_path: Option<String>,
    paste_text: Option<String>,
    encrypted: bool,
    passphrase: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ImportPreviewRow>, CommandError> {
    let vault_id = VaultId::new(PathBuf::from(&vault_path));
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let source = read_source(src_path, paste_text, encrypted, passphrase)?;
    let rows = begin_import_core(&mut guard, source)?;
    Ok(rows.iter().map(preview_row_to_dto).collect())
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn commit_import(
    vault_path: String,
    actions: Vec<ImportActionDto>,
    state: tauri::State<'_, AppState>,
) -> Result<ImportReportDto, CommandError> {
    let vault_id = VaultId::new(PathBuf::from(&vault_path));
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let core_actions: Vec<_> = actions.iter().map(action_from_dto).collect();
    let report = commit_import_core(&mut guard, &core_actions).await?;
    Ok(report_to_dto(report))
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn cancel_import(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = VaultId::new(PathBuf::from(&vault_path));
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;
    cancel_import_core(&mut guard);
    Ok(())
}
