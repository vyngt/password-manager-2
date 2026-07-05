//! Document import/export. Mirrors
//! `vedge-tauri/src/commands/document.rs`.
//!
//! The desktop flow uses the **path-based** pair
//! ([`import_document_from_path`] / [`export_document_to_path`]): the renderer
//! only ever passes native-dialog paths, and the backend does all file I/O with
//! `std::fs`. Decrypted document bytes never cross into WASM. The bytes-based
//! pair is retained for a future browser/CLI shell.

use serde::Serialize;

use vedge_ipc::{CommonMetaDto, ExportedDocumentDto};

use crate::api::call::{call, call_void};
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

/// Attach a file: the backend reads `src_path` with `std::fs`, encrypts it into
/// the sidecar blob, and creates a `Document` entry. Returns the new entry id.
/// Bytes never cross into WASM.
pub async fn import_document_from_path(
    vault_path: &str,
    src_path: &str,
    meta: &CommonMetaDto,
) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        src_path: &'a str,
        meta: &'a CommonMetaDto,
    }
    call(
        "import_document_from_path",
        &Args {
            vault_path,
            src_path,
            meta,
        },
    )
    .await
}

/// Export a `Document` entry: the backend decrypts the blob and writes the
/// plaintext straight to `dest_path` (a native save-dialog path). Bytes never
/// cross into WASM.
pub async fn export_document_to_path(
    vault_path: &str,
    entry_id: &str,
    dest_path: &str,
) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        entry_id: &'a str,
        dest_path: &'a str,
    }
    call_void(
        "export_document_to_path",
        &Args {
            vault_path,
            entry_id,
            dest_path,
        },
    )
    .await
}
