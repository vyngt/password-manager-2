//! Export command (slice 5.3a) — write the unlocked vault's entries to a
//! portable file.
//!
//! Two formats behind one command: an **encrypted** envelope (needs a
//! passphrase; opaque on disk) or a **plaintext** logins-only CSV (the shell
//! shows the honest ⑧ warning before writing it). Holds the session guard across
//! the write, like `backup_vault`.
//!
//! 🔴 The passphrase crosses the IPC boundary *in* (like the master password on
//! unlock) and is wrapped in `Zeroizing` immediately; nothing secret comes back.

// The `#[tauri::command]` expansion emits an `unreachable!` arm and a non-binding
// `let _ = …` on a `#[must_use]` value; allowed crate-wide in the other command
// modules for the same reason.
#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;
use zeroize::Zeroizing;

use vedge_core::domain::shared::VaultId;
use vedge_core::{ExportEntriesInput, ExportFormat, export_entries as export_entries_core};

use crate::dto::entry::entry_id_from_str;
use crate::dto::export::{ExportReportDto, export_report_to_dto};
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, encrypted))]
pub async fn export_entries(
    vault_path: String,
    dest_path: String,
    encrypted: bool,
    passphrase: Option<String>,
    spreadsheet_safe: bool,
    // Optional subset of entry ids (the selection / filter scope). `None` = all.
    entry_ids: Option<Vec<String>>,
    state: tauri::State<'_, AppState>,
) -> Result<ExportReportDto, CommandError> {
    let vault_id = VaultId::new(PathBuf::from(&vault_path));
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let format = if encrypted {
        let pass = passphrase
            .filter(|p| !p.is_empty())
            .ok_or_else(|| CommandError::Invalid("an export passphrase is required".into()))?;
        ExportFormat::Encrypted {
            passphrase: Zeroizing::new(pass),
        }
    } else {
        ExportFormat::Csv { spreadsheet_safe }
    };

    let report = export_entries_core(
        &guard,
        ExportEntriesInput {
            dest: PathBuf::from(dest_path),
            format,
            ids: entry_ids.map(|v| v.iter().map(|s| entry_id_from_str(s)).collect()),
        },
    )
    .await?;
    Ok(export_report_to_dto(report))
}
