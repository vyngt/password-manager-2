//! Maintenance pass. Mirrors `vedge-tauri/src/commands/maintenance.rs`.

use serde::Serialize;

use vedge_ipc::MaintenanceReportDto;

use crate::api::call::call;
use crate::api::error::ApiError;

pub async fn run_maintenance(vault_path: &str) -> Result<MaintenanceReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("run_maintenance", &Args { vault_path }).await
}
