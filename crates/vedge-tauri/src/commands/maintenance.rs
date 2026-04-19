//! `run_maintenance` — daily cleanup pass. The use case only reads
//! `session.config`, so we take the mutex as an immutable guard.
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
use vedge_core::run_maintenance as run_maintenance_core;

use crate::dto::misc::{MaintenanceReportDto, maintenance_report_to_dto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn run_maintenance(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<MaintenanceReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let report = run_maintenance_core(&guard).await?;
    Ok(maintenance_report_to_dto(report))
}
