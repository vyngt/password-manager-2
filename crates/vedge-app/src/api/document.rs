//! Document import/export. Mirrors
//! `vedge-tauri/src/commands/document.rs`.
//!
//! The desktop flow uses the **path-based** pair
//! ([`import_document_from_path`] / [`export_document_to_path`]): the renderer
//! only ever passes native-dialog paths, and the backend does all file I/O with
//! `std::fs`. Decrypted document bytes never cross into WASM.

use serde::Serialize;

use vedge_ipc::CommonMetaDto;

use crate::api::call::{call, call_void};
use crate::api::error::ApiError;

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
