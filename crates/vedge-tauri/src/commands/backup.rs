//! `backup_vault` — write a live-safe `.vbk` archive of an unlocked vault (5.2).
//!
//! Requires an unlocked session (the snapshot runs through its repo). The guard
//! is held across the archive write; see `commands/maintenance.rs` for the
//! `#![allow]` rationale.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::{BackupVaultInput, backup_vault as backup_vault_core};

use crate::dto::backup::{BackupReportDto, backup_report_to_dto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn backup_vault(
    vault_path: String,
    dest_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let report = backup_vault_core(
        &guard,
        BackupVaultInput {
            dest_archive: PathBuf::from(dest_path),
        },
    )
    .await?;
    Ok(backup_report_to_dto(report))
}
