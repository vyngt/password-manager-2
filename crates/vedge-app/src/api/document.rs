//! Document import/export. Mirrors
//! `vedge-tauri/src/commands/document.rs`.

use serde::Serialize;

use vedge_ipc::{CommonMetaDto, ExportedDocumentDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Import a document. Bytes cross as `Vec<u8>` (Tauri handles the
/// wire-format conversion).
pub async fn import_document(
    vault_path: &str,
    filename: &str,
    mime_type: &str,
    content: Vec<u8>,
    meta: &CommonMetaDto,
) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        filename: &'a str,
        mime_type: &'a str,
        content: Vec<u8>,
        meta: &'a CommonMetaDto,
    }
    call(
        "import_document",
        &Args {
            vault_path,
            filename,
            mime_type,
            content,
            meta,
        },
    )
    .await
}

pub async fn export_document(
    vault_path: &str,
    entry_id: &str,
) -> Result<ExportedDocumentDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
    }
    call(
        "export_document",
        &Args {
            vault_path,
            entry_id,
        },
    )
    .await
}
