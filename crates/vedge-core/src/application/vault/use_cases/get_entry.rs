//! `get_entry` — decrypt one entry and return its full plaintext `EntryPayload`.
//!
//! The sanctioned, user-initiated *reveal* path. Unlike `copy_field` (which
//! places a single field on the OS clipboard), this hands the whole
//! secret-bearing payload back to the caller so an edit form can prefill and
//! preserve existing values. Read-only: it mirrors `move_entry`'s decrypt
//! preamble and `copy_field`'s side-effects (audit `Viewed` + touch
//! `accessed_at`), but performs no re-encrypt.
//!
//! The returned `EntryPayload` keeps its secrets in `SecretString`, which
//! zeroizes on drop — the caller owns that lifetime.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;

#[derive(Debug)]
pub struct GetEntryInput {
    pub entry_id: EntryId,
}

#[instrument(skip_all, fields(entry_id = %input.entry_id))]
pub async fn get_entry(
    session: &mut VaultSession,
    input: GetEntryInput,
) -> Result<EntryPayload, VaultError> {
    // 1. Fetch the ciphertext row — the index doesn't hold secrets. A missing
    //    id surfaces as `VaultError::EntryNotFound`.
    let row = session.repo.get_entry(&input.entry_id).await?;

    // 2. Decrypt under the session KEK (shared helper; side-effects below).
    let payload = super::refs::decrypt_row_payload(session, &row)?;

    // An unknown entry has no editable/DTO representation — reject it the same
    // way create_entry/update_entry do.
    if matches!(payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot reveal an unknown entry".into(),
        ));
    }

    // 3. Side-effects after the crypto work. A reveal exposes secrets, so audit
    //    it as `Viewed` and touch `accessed_at`, consistent with copy_field.
    let when = now();
    session.repo.update_accessed_at(&row.id, when).await?;
    super::create_entry::append_audit(session, AuditAction::Viewed, Some(&row.id)).await?;

    if let Some(entry) = session.index.entries.get(&row.id).cloned() {
        let mut updated = entry;
        updated.accessed_at = Some(when);
        session.index.update_entry(updated);
    }

    Ok(payload)
}
