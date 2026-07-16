//! Import command wrappers (slice 5.3b). Mirrors `vedge-tauri/src/commands/import.rs`.
//!
//! The preview crosses **derivatives only** (Decision ⑦). The passphrase (for an
//! encrypted source) and the pasted CSV text (Decision ⑧) cross *in* and never
//! come back; the backend reads a picked file with `std::fs` — bytes never reach
//! WASM.

use serde::Serialize;

use vedge_ipc::{ImportActionDto, ImportPreviewRow, ImportReportDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Begin a staged import from a picked file (`src_path`) or pasted CSV text
/// (`paste_text`); returns the derivatives-only preview.
pub async fn begin(
    vault_path: &str,
    src_path: Option<&str>,
    paste_text: Option<&str>,
    encrypted: bool,
    passphrase: Option<&str>,
) -> Result<Vec<ImportPreviewRow>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        src_path: Option<&'a str>,
        paste_text: Option<&'a str>,
        encrypted: bool,
        passphrase: Option<&'a str>,
    }
    call(
        "begin_import",
        &Args {
            vault_path,
            src_path,
            paste_text,
            encrypted,
            passphrase,
        },
    )
    .await
}

/// Commit the picked rows; returns the import report.
pub async fn commit(
    vault_path: &str,
    actions: &[ImportActionDto],
) -> Result<ImportReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        actions: &'a [ImportActionDto],
    }
    call(
        "commit_import",
        &Args {
            vault_path,
            actions,
        },
    )
    .await
}

/// Discard an in-progress import — the backend drops the staging (zeroizing it).
pub async fn cancel(vault_path: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("cancel_import", &Args { vault_path }).await
}
