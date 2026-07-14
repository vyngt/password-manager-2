//! Document import/export.
//!
//! Two shapes:
//! - **bytes-based** ([`import_document`] / [`export_document`]) — the file
//!   bytes cross the IPC boundary (import takes `Vec<u8>`; export returns
//!   base64). Kept for a future browser/CLI shell.
//! - **path-based** ([`import_document_from_path`] / [`export_document_to_path`])
//!   — the desktop flow: bytes stay entirely in the backend, read from / written
//!   to a native-dialog-picked path with `std::fs`. Mirrors
//!   `emergency_kit::write_emergency_kit_pdf`; nothing decrypted crosses into
//!   WASM and no temp file is written.
//!
//! Both paths enforce the 50 MiB cap inside vedge-core (the path import also
//! stat-guards *before* reading, so an oversize file is never loaded).
//!
//! See `commands/vault.rs` for the `#![allow]` rationale.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::{Path, PathBuf};

use tracing::instrument;

use vedge_core::DOCUMENT_SIZE_LIMIT_BYTES;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::domain::shared::VaultId;
use vedge_core::{
    ImportDocumentInput, export_document as export_document_core,
    import_document as import_document_core,
};

use crate::dto::common::{CommonMetaDto, b64_encode, common_meta_from_dto};
use crate::dto::entry::entry_id_from_str;
use crate::dto::misc::ExportedDocumentDto;
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// The original filename to store in the payload, derived from the picked path.
/// Falls back to `"document"` for paths with no final component.
fn filename_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| "document".to_owned(), str::to_owned)
}

/// Best-effort MIME type from the file extension. We keep a small, dependency-free
/// table of common types and fall back to the generic binary type — the value is
/// stored for reference only (there is no in-app preview this slice).
fn mime_from_extension(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    };
    mime.to_owned()
}

/// Stat, size-gate, then read a file for import. The size check runs on the
/// stat'd length **before** the read, so an oversize file is rejected without
/// ever being loaded into memory.
fn read_document_for_import(src_path: &str) -> Result<(String, String, Vec<u8>), CommandError> {
    let path = Path::new(src_path);
    let len = std::fs::metadata(path)
        .map_err(|e| CommandError::Storage(format!("stat document: {e}")))?
        .len();
    if len > DOCUMENT_SIZE_LIMIT_BYTES {
        // Dedicated typed kind (not `Invalid`) so the frontend matches it
        // structurally — mirrors the use-case's `VaultError::DocumentTooLarge`.
        return Err(CommandError::DocumentTooLarge);
    }
    let filename = filename_from_path(path);
    let mime_type = mime_from_extension(path);
    let content =
        std::fs::read(path).map_err(|e| CommandError::Storage(format!("read document: {e}")))?;
    Ok((filename, mime_type, content))
}

/// Read the file at `src_path` and import it into the session as a `Document`
/// entry. Split out of the command wrapper so it can be tested without a Tauri
/// runtime.
async fn import_from_path_into(
    session: &mut VaultSession,
    src_path: &str,
    meta: CommonMetaDto,
) -> Result<String, CommandError> {
    let (filename, mime_type, content) = read_document_for_import(src_path)?;
    let id = import_document_core(
        session,
        ImportDocumentInput {
            filename,
            mime_type,
            content,
            meta: common_meta_from_dto(meta),
        },
    )
    .await?;
    Ok(id.into_string())
}

/// Export a `Document` entry's decrypted bytes straight to `dest_path`. Split
/// out of the command wrapper for the same testability reason.
async fn export_to_path_from(
    session: &VaultSession,
    entry_id: &str,
    dest_path: &str,
) -> Result<(), CommandError> {
    let id = entry_id_from_str(entry_id);
    let (_filename, bytes) = export_document_core(session, &id).await?;
    // `bytes` is a `Zeroizing<Vec<u8>>` — it drops (zeroizes) at scope end.
    std::fs::write(dest_path, &bytes)
        .map_err(|e| CommandError::Storage(format!("write document: {e}")))
}

#[tauri::command(rename_all = "snake_case")]
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
            meta: common_meta_from_dto(meta),
        },
    )
    .await?;
    Ok(id.into_string())
}

#[tauri::command(rename_all = "snake_case")]
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

/// Read a file from `src_path` and store it as an encrypted `Document` entry.
/// The bytes are read by the backend with `std::fs` and never cross into WASM.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, src_path = %src_path))]
pub async fn import_document_from_path(
    vault_path: String,
    src_path: String,
    meta: CommonMetaDto,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let mut guard = handle.lock().await;
    import_from_path_into(&mut guard, &src_path, meta).await
}

/// Decrypt a `Document` entry and write its bytes to `dest_path` (a native
/// save-dialog path). The plaintext is written by the backend with `std::fs`
/// and never crosses into WASM.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path, entry_id = %entry_id))]
pub async fn export_document_to_path(
    vault_path: String,
    entry_id: String,
    dest_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;
    export_to_path_from(&guard, &entry_id, &dest_path).await
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )]

    use std::fs::File;
    use std::path::Path;

    use vedge_ipc::EntryTypeDto;

    use super::{
        CommonMetaDto, DOCUMENT_SIZE_LIMIT_BYTES, export_to_path_from, filename_from_path,
        import_from_path_into, mime_from_extension, read_document_for_import,
    };
    use crate::error::CommandError;
    use crate::test_support::unlocked_vault;

    fn doc_meta(name: &str) -> CommonMetaDto {
        CommonMetaDto {
            name: name.to_owned(),
            entry_type: EntryTypeDto::Document,
            url: None,
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            notes: None,
            color: None,
            icon: None,
            sort_order: 0,
        }
    }

    #[test]
    fn mime_from_extension_maps_known_and_unknown() {
        assert_eq!(
            mime_from_extension(Path::new("a/b/report.pdf")),
            "application/pdf"
        );
        assert_eq!(mime_from_extension(Path::new("IMG.JPG")), "image/jpeg");
        assert_eq!(mime_from_extension(Path::new("notes.md")), "text/markdown");
        assert_eq!(
            mime_from_extension(Path::new("archive.unknownext")),
            "application/octet-stream"
        );
        assert_eq!(
            mime_from_extension(Path::new("noext")),
            "application/octet-stream"
        );
    }

    #[test]
    fn filename_from_path_takes_final_component() {
        assert_eq!(
            filename_from_path(Path::new("/tmp/dir/report.pdf")),
            "report.pdf"
        );
        assert_eq!(filename_from_path(Path::new("bare")), "bare");
        // Backslash is a path separator only on Windows — on Unix the whole
        // `C:\Users\me\key.txt` is one component. Assert the Windows split only
        // where it actually applies (surfaced by the Linux CI, slice 3.8).
        #[cfg(windows)]
        assert_eq!(
            filename_from_path(Path::new(r"C:\Users\me\key.txt")),
            "key.txt"
        );
    }

    #[test]
    fn read_document_for_import_rejects_oversize_before_reading() {
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("huge.bin");
        // Sparse file: sets the *logical* length without writing 50 MiB. The
        // stat guard reads this length and rejects before `std::fs::read`.
        let f = File::create(&big).unwrap();
        f.set_len(DOCUMENT_SIZE_LIMIT_BYTES + 1).unwrap();
        drop(f);

        match read_document_for_import(big.to_str().unwrap()) {
            Err(CommandError::DocumentTooLarge) => {}
            other => panic!("expected DocumentTooLarge, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn import_from_path_roundtrips_bytes_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "docs.vedge").await;

        let original: &[u8] = b"hello, this is an encrypted document body\x00\x01\x02";
        let src = dir.path().join("hello.txt");
        std::fs::write(&src, original).unwrap();

        let handle = state.get_session(&vault_id).unwrap();

        let id = {
            let mut guard = handle.lock().await;
            import_from_path_into(&mut guard, src.to_str().unwrap(), doc_meta("Hello"))
                .await
                .unwrap()
        };
        assert!(!id.is_empty());

        // On disk, the blob lives inside the vault home's `blobs/` dir (slice 5.2.0)
        // and is NOT the plaintext (it's ciphertext).
        let blob = dir
            .path()
            .join("docs.vedge")
            .join("blobs")
            .join(format!("{id}.blob"));
        assert!(blob.exists(), "expected sidecar blob at {blob:?}");
        assert_ne!(std::fs::read(&blob).unwrap(), original);

        // Export it back out to a chosen path — bytes byte-equal the original.
        let dest = dir.path().join("exported.txt");
        {
            let guard = handle.lock().await;
            export_to_path_from(&guard, &id, dest.to_str().unwrap())
                .await
                .unwrap();
        }
        assert_eq!(std::fs::read(&dest).unwrap(), original);
    }
}
