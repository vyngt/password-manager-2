//! Shared helpers used by the vault use cases — reference-integrity checks and
//! the canonical row-decrypt sequence.
//!
//! Every entry is allowed to name a parent folder and carry tag IDs. The
//! in-memory [`VaultIndex`](crate::domain::vault::index::VaultIndex) is the
//! authority on what exists right now in this session — so we validate
//! against it before writing anything to disk. A reference to a folder or
//! tag that was just deleted is treated as a caller bug, not silently
//! accepted.

use crate::application::vault::ports::crypto::CryptoProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::EntryId;
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::crypto_constants::DEK_LEN;
use crate::domain::vault::entities::EntryRow;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType};

/// The canonical entry-decrypt sequence — `entry_aad → decrypt_entry → parse`.
/// Extracted (slice 4.3) from the copies inlined across the read/update paths
/// (`unlock_vault`, `get_entry`, `copy_field`, `reveal_totp`, `update_entry`); the
/// bulk health scan reuses it rather than writing another. **Side-effect-free**:
/// it writes no audit row and does not touch `accessed_at` — the caller owns those.
///
/// The metadata-mutation family (`set_favorite` / `set_tags` / `set_sort_order` /
/// `move_entry`) still inlines the same triplet inside a decrypt→mutate→re-encrypt
/// flow; routing those through here (or a higher-level `mutate_meta` helper) is a
/// tracked follow-up — see the 4.3 tracking note.
///
/// Takes the already-unwrapped `dek` so `unlock_vault` (which holds the KEK as a
/// parameter, before any session exists) and `update_entry` (which reuses the DEK
/// it just unwrapped for re-encryption) can both share it.
pub(super) fn decrypt_row_with_dek(
    crypto: &dyn CryptoProvider,
    dek: &[u8; DEK_LEN],
    row: &EntryRow,
) -> Result<EntryPayload, VaultError> {
    let aad = entry_aad(&row.id, row.version)?;
    let plaintext = crypto.decrypt_entry(dek, &row.nonce, &row.ciphertext, &aad)?;
    EntryPayload::from_decrypted_json(&plaintext)
}

/// Unwrap the row's DEK under the session KEK, then decrypt. The session-scoped
/// convenience over [`decrypt_row_with_dek`] for use cases that hold a
/// `VaultSession`. Side-effect-free (no audit, no `accessed_at`).
pub(super) fn decrypt_row_payload(
    session: &VaultSession,
    row: &EntryRow,
) -> Result<EntryPayload, VaultError> {
    let dek = session
        .crypto
        .unwrap_dek(&row.dek_wrapped, session.kek.expose())?;
    decrypt_row_with_dek(session.crypto.as_ref(), &dek, row)
}

pub(super) fn validate_common_meta(
    session: &VaultSession,
    meta: &CommonMeta,
) -> Result<(), VaultError> {
    if let Some(folder_id) = meta.folder_id.as_ref() {
        validate_folder(session, folder_id)?;
    }
    for tag_id in &meta.tag_ids {
        if !session.index.tags.contains_key(tag_id) {
            return Err(VaultError::TagNotFound(tag_id.clone()));
        }
    }
    Ok(())
}

pub(super) fn validate_folder(
    session: &VaultSession,
    folder_id: &EntryId,
) -> Result<(), VaultError> {
    let folder: &IndexEntry = session
        .index
        .entries
        .get(folder_id)
        .ok_or_else(|| VaultError::FolderNotFound(folder_id.clone()))?;
    if folder.entry_type != EntryType::Folder {
        return Err(VaultError::FolderNotFound(folder_id.clone()));
    }
    Ok(())
}
