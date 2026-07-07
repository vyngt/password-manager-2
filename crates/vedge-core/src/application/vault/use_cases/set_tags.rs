//! `set_tags` — replace an entry's `tag_ids`.
//!
//! Like `set_favorite`/`move_entry`, the tag list lives inside the encrypted
//! `CommonMeta`, so this decrypts → replaces `meta.tag_ids` → re-encrypts with a
//! fresh nonce + bumped version, preserving the DEK. It works for **any** entry
//! type (Document included — its blob sidecar is untouched), which is why
//! tagging goes through this dedicated path rather than the edit form: no
//! secrets are returned to the caller, and non-editable types can still be
//! tagged. Referential integrity is intentionally not enforced — the index
//! tolerates dangling ids and the UI resolves what it can.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, TagId, now};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::EntryPayload;

#[instrument(skip_all, fields(entry_id = %entry_id, tags = tag_ids.len()))]
pub async fn set_tags(
    session: &mut VaultSession,
    entry_id: &EntryId,
    tag_ids: Vec<TagId>,
) -> Result<(), VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }

    let existing = session.repo.get_entry(entry_id).await?;
    let dek = session
        .crypto
        .unwrap_dek(&existing.dek_wrapped, session.kek.expose())?;
    let aad_old = entry_aad(entry_id, existing.version)?;
    let plaintext =
        session
            .crypto
            .decrypt_entry(&dek, &existing.nonce, &existing.ciphertext, &aad_old)?;
    let mut payload = EntryPayload::from_decrypted_json(&plaintext)?;
    drop(plaintext);

    if matches!(payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot tag an unknown entry".into(),
        ));
    }
    payload.meta_mut().tag_ids = tag_ids;

    let new_version = existing
        .version
        .checked_add(1)
        .ok_or_else(|| VaultError::MalformedPayload("entry version overflow".into()))?;
    let aad_new = entry_aad(entry_id, new_version)?;
    let payload_bytes = payload.to_encryptable_json()?;
    let (nonce, ciphertext) = session
        .crypto
        .encrypt_entry(&dek, &payload_bytes, &aad_new)?;
    drop(dek);

    let when = now();
    let mut row = existing;
    row.version = new_version;
    row.nonce = nonce;
    row.ciphertext = ciphertext;
    row.updated_at = when;
    session.repo.update_entry(&row).await?;

    super::create_entry::append_audit(session, AuditAction::Updated, Some(entry_id)).await?;

    let index_entry = IndexEntry::from_payload(&payload, &row);
    session.index.update_entry(index_entry);
    Ok(())
}
