//! Document import/export. Both enforce the 50 MiB cap inside vedge-core —
//! the shell passes bytes through and encodes the export to base64 so it
//! round-trips cleanly through Tauri's JSON IPC.
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::{export_document as export_document_core, import_document as import_document_core,
    ImportDocumentInput,
};

use crate::dto::common::{b64_encode, CommonMetaDto};
use crate::dto::entry::entry_id_from_str;
use crate::dto::misc::ExportedDocumentDto;
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, filename = %filename, bytes = content.len()))]
pub async fn import_document(
    vault_path: String,
    filename: String,
    mime_type: String,
    content: Vec<u8>,
    meta: CommonMetaDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;

    let id = import_document_core(
        &mut guard,
        ImportDocumentInput {
            filename,
            mime_type,
            content,
            meta: meta.into_domain(),
        },
    )
    .await?;
    Ok(id.into_string())
}

#[tauri::command]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn export_document(
    vault_path: String,
    entry_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportedDocumentDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let id = entry_id_from_str(&entry_id);
    let (filename, bytes) = export_document_core(&guard, &id).await?;
    // `bytes` is a `Zeroizing<Vec<u8>>` — the base64 copy we hand back to
    // JS lives only for the command call. The zeroizing original drops
    // at the end of this scope.
    Ok(ExportedDocumentDto {
        filename,
        content_b64: b64_encode(&bytes),
    })
}
