//! `update_entry` — re-encrypt an existing row with a new payload, keeping
//! the per-entry DEK stable but rotating nonce + version.
//!
//! Security invariants:
//!
//! - **Version strictly increases.** AAD = `entry_id || version`; a replayed
//!   ciphertext from an older version cannot authenticate against the new
//!   version's AAD.
//! - **Fresh nonce on every write** — even when the payload bytes are
//!   semantically identical to the previous version.
//! - **Same DEK** — the per-entry key is stable across the entry's lifetime;
//!   rotating it would invalidate blob references keyed on the same DEK for
//!   Document entries. DEK rotation is a `ChangePassword` concern (re-wrap
//!   under new KEK), not an update concern.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{now, EntryId};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::EntryPayload;

pub struct UpdateEntryInput {
    pub entry_id: EntryId,
    /// Complete replacement payload. Callers own the merge logic (read the
    /// current payload, mutate, pass back).
    pub payload: EntryPayload,
}

#[instrument(skip_all, fields(entry_id = %input.entry_id))]
pub async fn update_entry(
    session: &mut VaultSession,
    input: UpdateEntryInput,
) -> Result<(), VaultError> {
    if matches!(input.payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot update to an unknown entry type".into(),
        ));
    }
    // Must exist in the index (authoritative read source).
    if !session.index.entries.contains_key(&input.entry_id) {
        return Err(VaultError::EntryNotFound(input.entry_id.clone()));
    }
    super::refs::validate_common_meta(session, input.payload.meta())?;

    // Fetch the DB row to get the current DEK + version.
    let existing = session.repo.get_entry(&input.entry_id).await?;
    let new_version = existing
        .version
        .checked_add(1)
        .ok_or_else(|| VaultError::MalformedPayload("entry version overflow".into()))?;

    // Unwrap the DEK into a zeroizing buffer — used only in this scope.
    let dek = session
        .crypto
        .unwrap_dek(&existing.dek_wrapped, session.kek.expose())?;

    let new_payload = input.payload;
    let payload_bytes = new_payload.to_encryptable_json()?;

    let aad = entry_aad(&input.entry_id, new_version)?;
    let (nonce, ciphertext) = session.crypto.encrypt_entry(&dek, &payload_bytes, &aad)?;
    // `dek` drops here, zeroizing. The wrapped DEK on disk is unchanged.

    let when = now();
    let mut row = existing;
    row.version = new_version;
    row.nonce = nonce;
    row.ciphertext = ciphertext;
    row.updated_at = when;
    session.repo.update_entry(&row).await?;

    super::create_entry::append_audit(session, AuditAction::Updated, Some(&input.entry_id))
        .await?;

    let index_entry = IndexEntry::from_payload(&new_payload, &row);
    session.index.update_entry(index_entry);
    Ok(())
}
