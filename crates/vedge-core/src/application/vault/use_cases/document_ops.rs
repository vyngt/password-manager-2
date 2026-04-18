//! `import_document`, `export_document`.
//!
//! Documents live as two things: an encrypted payload row (metadata +
//! `blob_nonce`) inside the `.vdb` and a sidecar ciphertext file under
//! `.vedge_blobs/{entry_id}.blob`. The DEK is shared between them.
//!
//! ## Ordering on import
//!
//! 1. Size gate — reject >50 MiB before touching disk.
//! 2. Fresh ID + DEK.
//! 3. Write the blob first. The blob nonce returned is embedded in the
//!    payload.
//! 4. Encrypt and insert the entry row. If this fails (rare — disk full
//!    mid-flow), the blob is orphaned and [`run_maintenance`] will reap it
//!    on the next daily pass.
//! 5. Audit + index insert.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{now, EntryId};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::crypto_constants::NONCE_LEN;
use crate::domain::vault::entities::{AuditAction, EntryRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::{CommonMeta, DocumentPayload, EntryPayload, EntryType};
use zeroize::Zeroizing;

/// 50 MiB cap, per spec. Matches `u64::from(50) * 1024 * 1024` exactly.
pub const DOCUMENT_SIZE_LIMIT_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug)]
pub struct ImportDocumentInput {
    pub filename: String,
    pub mime_type: String,
    /// Raw file bytes. Rejected up-front if longer than
    /// [`DOCUMENT_SIZE_LIMIT_BYTES`].
    pub content: Vec<u8>,
    /// Name / tags / folder from the caller. `entry_type` is forced to
    /// `Document`; any value here is ignored.
    pub meta: CommonMeta,
}

#[instrument(skip_all, fields(filename = %input.filename, bytes = input.content.len()))]
pub async fn import_document(
    session: &mut VaultSession,
    input: ImportDocumentInput,
) -> Result<EntryId, VaultError> {
    let size = input.content.len() as u64;
    if size > DOCUMENT_SIZE_LIMIT_BYTES {
        return Err(VaultError::DocumentTooLarge {
            size,
            limit: DOCUMENT_SIZE_LIMIT_BYTES,
        });
    }

    let entry_id = EntryId::new();
    let version: i64 = 1;

    // Validate tag/folder references before we write anything.
    let mut meta = input.meta;
    meta.entry_type = EntryType::Document;
    super::refs::validate_common_meta(session, &meta)?;

    // Fresh DEK — used for both blob and entry.
    let dek = session.crypto.generate_dek();

    // Write the blob first. `write_blob` returns the nonce it used; we embed
    // that in the payload so `ExportDocument` can find it.
    let blob_nonce: [u8; NONCE_LEN] = session
        .blob
        .write_blob(&entry_id, &dek, &input.content)
        .await?;

    // Build + encrypt the payload.
    let payload = EntryPayload::Document(DocumentPayload {
        meta,
        filename: input.filename,
        mime_type: input.mime_type,
        size_bytes: size,
        blob_nonce,
    });
    let payload_bytes = payload.to_encryptable_json()?;

    let aad = entry_aad(&entry_id, version)?;
    let (nonce, ciphertext) = session.crypto.encrypt_entry(&dek, &payload_bytes, &aad)?;
    let dek_wrapped = session.crypto.wrap_dek(&dek, session.kek.expose())?;
    drop(dek);

    let when = now();
    let row = EntryRow {
        id: entry_id.clone(),
        version,
        cipher_suite: session.config.preferred_cipher_suite,
        dek_wrapped,
        nonce,
        ciphertext,
        created_at: when,
        updated_at: when,
        accessed_at: None,
        is_trashed: false,
        trashed_at: None,
    };
    session.repo.insert_entry(&row).await?;

    super::create_entry::append_audit(session, AuditAction::Created, Some(&entry_id)).await?;

    let idx = IndexEntry::from_payload(&payload, &row);
    session.index.insert_entry(idx);
    Ok(entry_id)
}

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn export_document(
    session: &VaultSession,
    entry_id: &EntryId,
) -> Result<(String, Zeroizing<Vec<u8>>), VaultError> {
    let row = session.repo.get_entry(entry_id).await?;
    let dek = session
        .crypto
        .unwrap_dek(&row.dek_wrapped, session.kek.expose())?;
    let aad = entry_aad(entry_id, row.version)?;
    let plaintext = session
        .crypto
        .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)?;
    let payload = EntryPayload::from_decrypted_json(&plaintext)?;
    drop(plaintext);

    let EntryPayload::Document(doc) = payload else {
        return Err(VaultError::FieldNotApplicable);
    };

    let bytes = session
        .blob
        .read_blob(entry_id, &dek, &doc.blob_nonce)
        .await?;
    drop(dek);

    Ok((doc.filename, bytes))
}
