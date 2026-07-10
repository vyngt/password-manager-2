//! Hardened clipboard write for renderer-generated secrets. Mirrors
//! `vedge-tauri/src/commands/clipboard.rs` (`copy_text`).

use serde::Serialize;

use crate::api::call::call_void;
use crate::api::error::ApiError;

/// Place `text` on the OS clipboard through the hardened native provider (the
/// same one `copy_field` uses — no-history / no-cloud exclusion hints) and
/// schedule the background auto-clear. `clear_after_secs = None` uses the
/// backend default (30 s). The secret crosses IPC once and is zeroized
/// native-side after the write.
pub async fn copy_text(text: &str, clear_after_secs: Option<u32>) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        text: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        clear_after_secs: Option<u32>,
    }
    call_void(
        "copy_text",
        &Args {
            text,
            clear_after_secs,
        },
    )
    .await
}
