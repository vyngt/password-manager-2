//! Shared reference-integrity checks used by every write-path use case.
//!
//! Every entry is allowed to name a parent folder and carry tag IDs. The
//! in-memory [`VaultIndex`](crate::domain::vault::index::VaultIndex) is the
//! authority on what exists right now in this session — so we validate
//! against it before writing anything to disk. A reference to a folder or
//! tag that was just deleted is treated as a caller bug, not silently
//! accepted.

use crate::application::vault::session::VaultSession;
use crate::domain::shared::EntryId;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::{CommonMeta, EntryType};

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
