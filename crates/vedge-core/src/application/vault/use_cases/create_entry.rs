//! `create_entry` — persist a fresh entry and its index projection.
//!
//! Security-relevant behaviour enforced here:
//!
//! - Fresh `EntryId`, fresh DEK, fresh nonce — every write uses new
//!   material. No reuse.
//! - `version = 1` on creation; AAD binds ciphertext to `(id, version)`.
//! - DEK is wrapped under the session's KEK and zeroized immediately.
//! - Decrypted plaintext bytes exit scope before the row is inserted —
//!   the only long-lived reference to the plaintext is the caller-owned
//!   `EntryPayload` (which holds its secrets in `SecretString`).
//! - `folder_id` / `tag_ids` carried inside the payload are validated
//!   against the in-memory `VaultIndex` — callers cannot pin a dangling
//!   folder or tag reference into a stored entry.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::{AuditAction, AuditEvent, EntryRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::EntryPayload;

#[derive(Debug)]
pub struct CreateEntryInput {
    pub payload: EntryPayload,
}

#[derive(Debug)]
pub struct CreateEntryOutput {
    pub entry_id: EntryId,
}

#[instrument(skip_all, fields(entry_type = ?input.payload.entry_type()))]
pub async fn create_entry(
    session: &mut VaultSession,
    input: CreateEntryInput,
) -> Result<CreateEntryOutput, VaultError> {
    if matches!(input.payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot create an entry of unknown type".into(),
        ));
    }
    super::refs::validate_common_meta(session, input.payload.meta())?;

    // Fresh identity + version.
    let entry_id = EntryId::new();
    let version: i64 = 1;

    // A fresh entry's secret material is, by definition, new — stamp its age
    // now (slice 4.3). Owned by the write path; the frontend never supplies it.
    let when = now();
    let mut rewritten = input.payload;
    rewritten.meta_mut().secret_changed_at = Some(when);

    // Serialize under a named boundary so callers can grep for encryption
    // sites. `to_encryptable_json` returns `Zeroizing<Vec<u8>>`.
    let payload_bytes = rewritten.to_encryptable_json()?;

    // Fresh DEK — wraps immediately under the session KEK.
    let dek = session.crypto.generate_dek();
    let aad = entry_aad(&entry_id, version)?;
    let (nonce, ciphertext) = session.crypto.encrypt_entry(&dek, &payload_bytes, &aad)?;
    let dek_wrapped = session.crypto.wrap_dek(&dek, session.kek.expose())?;
    // `dek` and `payload_bytes` go out of scope at the end of this fn,
    // zeroizing their buffers.

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

    // Audit after DB insert succeeds — audit rows should reflect effects
    // that actually landed.
    append_audit(session, AuditAction::Created, Some(&entry_id)).await?;

    // Update in-memory index. Build the IndexEntry from the same payload
    // we just serialized, using `rewritten` (still owned here) to avoid a
    // redundant decrypt.
    let index_entry = IndexEntry::from_payload(&rewritten, &row);
    session.index.insert_entry(index_entry);
    drop(rewritten);

    Ok(CreateEntryOutput { entry_id })
}

pub(super) async fn append_audit(
    session: &VaultSession,
    action: AuditAction,
    entry_id: Option<&EntryId>,
) -> Result<(), VaultError> {
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: entry_id.cloned(),
        action,
        occurred_at: now(),
        device_id: None,
    };
    session.repo.append_audit(&event).await
}
