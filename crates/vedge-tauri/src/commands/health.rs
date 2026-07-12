//! Password-health command (slice 4.3): the vault-wide secret scan.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale (macro expansion of
//! `#[tauri::command]` trips `unreachable` / `let_underscore_must_use`; holding
//! the session mutex trips `significant_drop_tightening`).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::scan_health as scan_health_core;

use crate::dto::health::{
    HealthReportDto, HealthScanInputDto, health_report_to_dto, health_scan_input_from_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Run a vault-wide password-health scan.
///
/// Decrypts every entry server-side and returns **only derivatives** (zxcvbn
/// scores, reuse-group ordinals, ages) — no secret crosses to WASM. Audit-silent
/// per entry; emits exactly one `HealthScanned` row. The slowest command in the
/// app (a full-vault decrypt), so the caller shows a spinner.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn scan_health(
    vault_path: String,
    input: HealthScanInputDto,
    state: tauri::State<'_, AppState>,
) -> Result<HealthReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let report = scan_health_core(&guard, health_scan_input_from_dto(input)).await?;
    Ok(health_report_to_dto(&report))
}
