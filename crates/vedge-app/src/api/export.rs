//! Export command wrapper (slice 5.3a). Mirrors `vedge-tauri/src/commands/export.rs`.
//!
//! One command, two formats: an encrypted envelope (all entry types, sealed with
//! a passphrase) or a plaintext logins-only CSV. The passphrase crosses in and
//! never comes back.

use serde::Serialize;

use vedge_ipc::ExportReportDto;

use crate::api::call::call;
use crate::api::error::ApiError;

/// Export the unlocked vault's entries to `dest_path`.
///
/// `encrypted = true` seals an envelope with `passphrase` (all entry types);
/// `encrypted = false` writes a **plaintext** logins-only CSV, with
/// `spreadsheet_safe` apostrophe-prefixing formula-risky cells.
pub async fn export(
    vault_path: &str,
    dest_path: &str,
    encrypted: bool,
    passphrase: Option<&str>,
    spreadsheet_safe: bool,
) -> Result<ExportReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        dest_path: &'a str,
        encrypted: bool,
        passphrase: Option<&'a str>,
        spreadsheet_safe: bool,
    }
    call(
        "export_entries",
        &Args {
            vault_path,
            dest_path,
            encrypted,
            passphrase,
            spreadsheet_safe,
        },
    )
    .await
}
