//! Emergency Kit export. Mirrors
//! `vedge-tauri/src/commands/emergency_kit.rs`.

use serde::Serialize;

use vedge_ipc::EmergencyKitDto;

use crate::api::call::call;
use crate::api::error::ApiError;

pub async fn export_emergency_kit(vault_path: &str) -> Result<EmergencyKitDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("export_emergency_kit", &Args { vault_path }).await
}

/// Returns the raw PDF bytes. Shell-side PDF rendering is idempotent —
/// each call produces a fresh document.
pub async fn emergency_kit_pdf(vault_path: &str) -> Result<Vec<u8>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("emergency_kit_pdf", &Args { vault_path }).await
}
