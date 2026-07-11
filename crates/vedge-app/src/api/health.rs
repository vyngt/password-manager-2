//! Password-health command wrapper (slice 4.3b). Mirrors
//! `vedge-tauri/src/commands/health.rs`.

use serde::Serialize;
use vedge_ipc::{HealthReportDto, HealthScanInputDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Run the vault-wide secret-health scan. Decrypts every entry backend-side and
/// returns **only non-invertible derivatives** (weak score / reuse group / age);
/// no secret and no reuse digest crosses the boundary. Audit-silent per entry,
/// one `HealthScanned` row per run. This is the slowest operation in the app.
pub async fn scan_health(
    vault_path: &str,
    input: &HealthScanInputDto,
) -> Result<HealthReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        input: &'a HealthScanInputDto,
    }
    call("scan_health", &Args { vault_path, input }).await
}
