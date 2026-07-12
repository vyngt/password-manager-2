//! Maintenance command wrapper (slice 4.6a). Mirrors
//! `vedge-tauri/src/commands/maintenance.rs`.
//!
//! The same cleanup also runs automatically on a 6 h schedule (see the shell's
//! `scheduler`); this wrapper backs the Settings → Maintenance "Run now" button.

use serde::Serialize;
use vedge_ipc::MaintenanceReportDto;

use crate::api::call::call;
use crate::api::error::ApiError;

/// Run the vault's cleanup pass now: purge trash past its retention window, trim
/// the audit log past `audit_retention_days`, and reap orphaned document blobs.
/// Returns the counts deleted.
pub async fn run_maintenance(vault_path: &str) -> Result<MaintenanceReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("run_maintenance", &Args { vault_path }).await
}
