//! Emergency Kit export. Mirrors
//! `vedge-tauri/src/commands/emergency_kit.rs`.

use serde::Serialize;

use crate::api::call::call_void;
use crate::api::error::ApiError;

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
