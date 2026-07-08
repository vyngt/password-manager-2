//! Emergency Kit export. Mirrors
//! `vedge-tauri/src/commands/emergency_kit.rs`.

use serde::Serialize;

use vedge_ipc::EmergencyKitDto;

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

#[allow(dead_code)] // wrapper not yet called by UI
pub async fn export_emergency_kit(vault_path: &str) -> Result<EmergencyKitDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("export_emergency_kit", &Args { vault_path }).await
}

/// Returns the raw PDF bytes. Shell-side PDF rendering is idempotent —
/// each call produces a fresh document.
#[allow(dead_code)] // wrapper not yet called by UI (the path-based `write_pdf` is used)
pub async fn emergency_kit_pdf(vault_path: &str) -> Result<Vec<u8>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("emergency_kit_pdf", &Args { vault_path }).await
}

/// Render the Emergency Kit PDF and write it to `dest_path` (chosen via the
/// native save dialog). The shell writes the file with `std::fs`.
pub async fn write_pdf(vault_path: &str, dest_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        dest_path: &'a str,
    }
    call_void(
        "write_emergency_kit_pdf",
        &Args {
            vault_path,
            dest_path,
        },
    )
    .await
}
