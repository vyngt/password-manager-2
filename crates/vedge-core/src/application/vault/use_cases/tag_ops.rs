//! `create_tag`, `rename_tag`, `delete_tag` — share one tag-serialization
//! helper and the normalization rule, so they live in the same file.
//!
//! ## Normalization
//!
//! Tags are stored lowercased, trimmed, with internal whitespace collapsed to
//! single spaces. The UI should show what the user typed during entry, but
//! the *stored* form is canonical so duplicate detection and search are
//! case-insensitive.
//!
//! ## Encryption model
//!
//! Tags encrypt under the session KEK directly (no per-row DEK). The AAD
//! is the tag ULID bytes; see [`crate::domain::vault::aad::tag_aad`].
//!
//! ## Rename is O(1)
//!
//! Renaming a tag updates one row — the tag row itself. Entries reference
//! tags by `TagId`, not by name, so the thousands of entries carrying a
//! renamed tag are *untouched*. This is the payoff for keeping tags in a
//! separate table.
//!
//! ## Delete is O(entries carrying the tag)
//!
//! Deleting a tag must remove its ID from every entry that carries it.
//! Those entries are re-encrypted (fresh nonce + bumped version). If we
//! crash halfway, the vault is still consistent — some entries have the
//! tag removed, others don't, and the tag row is still present. Re-running
//! `delete_tag` resumes. Spec-accepted tradeoff (vs. introducing a
//! transaction boundary on `VaultRepository`).

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{TagId, now};
use crate::domain::vault::aad::{entry_aad, tag_aad};
use crate::domain::vault::entities::{AuditAction, TagRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::index::{IndexEntry, TagMeta};
use crate::domain::vault::payloads::{EntryPayload, TagPayload};

/// Normalize a tag name per the spec: lowercase, trim, collapse internal
/// whitespace to single spaces.
#[must_use]
pub fn normalize_tag_name(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut last_was_ws = false;
    for c in lower.chars() {
        if c.is_whitespace() {
            if !last_was_ws && !out.is_empty() {
                out.push(' ');
            }
            last_was_ws = true;
        } else {
            out.push(c);
            last_was_ws = false;
        }
    }
    // Trailing whitespace from the collapse above.
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

#[instrument(skip_all, fields(name = name))]
pub async fn create_tag(
    session: &mut VaultSession,
    name: &str,
    color: Option<String>,
) -> Result<TagId, VaultError> {
    let normalized = normalize_tag_name(name);

    // Idempotent: return the existing ID if the same normalized name is
    // already present.
    if let Some((existing_id, _)) = session
        .index
        .tags
        .iter()
        .find(|(_, t)| t.name == normalized)
    {
        return Ok(existing_id.clone());
    }

    let id = TagId::new();
    let payload = TagPayload {
        name: normalized.clone(),
        color: color.clone(),
        sort_order: u32::try_from(session.index.tags.len()).unwrap_or(u32::MAX),
    };
    let bytes = serde_json::to_vec(&payload)
        .map_err(|e| VaultError::MalformedPayload(format!("tag serialize: {e}")))?;
    let aad = tag_aad(&id)?;
    let (nonce, ciphertext) = session
        .crypto
        .encrypt_tag(session.kek.expose(), &bytes, &aad)?;

    let when = now();
    let row = TagRow {
        id: id.clone(),
        nonce,
        ciphertext,
        dek_wrapped: None,
        created_at: when,
        updated_at: when,
    };
    session.repo.insert_tag(&row).await?;

    super::create_entry::append_audit(session, AuditAction::TagCreated, None).await?;

    session.index.insert_tag(TagMeta {
        id: id.clone(),
        name: normalized,
        color,
        sort_order: payload.sort_order,
    });
    Ok(id)
}

#[instrument(skip_all, fields(tag_id = %tag_id))]
pub async fn rename_tag(
    session: &mut VaultSession,
    tag_id: &TagId,
    new_name: &str,
) -> Result<(), VaultError> {
    let current = session
        .index
        .tags
        .get(tag_id)
        .ok_or_else(|| VaultError::TagNotFound(tag_id.clone()))?
        .clone();
    let new_normalized = normalize_tag_name(new_name);
    if current.name == new_normalized {
        return Ok(()); // no-op
    }

    // Serialize an updated TagPayload with the same color + sort order.
    let payload = TagPayload {
        name: new_normalized.clone(),
        color: current.color.clone(),
        sort_order: current.sort_order,
    };
    let bytes = serde_json::to_vec(&payload)
        .map_err(|e| VaultError::MalformedPayload(format!("tag serialize: {e}")))?;
    let aad = tag_aad(tag_id)?;
    let (nonce, ciphertext) = session
        .crypto
        .encrypt_tag(session.kek.expose(), &bytes, &aad)?;

    let when = now();
    let row = TagRow {
        id: tag_id.clone(),
        nonce,
        ciphertext,
        dek_wrapped: None,
        created_at: when, // not updated by caller intent; overwritten by update_tag
        updated_at: when,
    };
    session.repo.update_tag(&row).await?;

    super::create_entry::append_audit(session, AuditAction::TagRenamed, None).await?;

    session.index.rename_tag(tag_id, new_normalized);
    Ok(())
}

#[instrument(skip_all, fields(tag_id = %tag_id))]
pub async fn delete_tag(session: &mut VaultSession, tag_id: &TagId) -> Result<(), VaultError> {
    if !session.index.tags.contains_key(tag_id) {
        return Err(VaultError::TagNotFound(tag_id.clone()));
    }

    // Snapshot the list of referencing entries — we iterate while the index
    // is still live.
    let affected: Vec<_> = session
        .index
        .tag_index
        .get(tag_id)
        .cloned()
        .unwrap_or_default();

    for entry_id in affected {
        // Fetch → decrypt → drop tag_id → re-encrypt → update.
        let existing = session.repo.get_entry(&entry_id).await?;
        let dek = session
            .crypto
            .unwrap_dek(&existing.dek_wrapped, session.kek.expose())?;
        let aad_old = entry_aad(&entry_id, existing.version)?;
        let plaintext =
            session
                .crypto
                .decrypt_entry(&dek, &existing.nonce, &existing.ciphertext, &aad_old)?;
        let mut payload = EntryPayload::from_decrypted_json(&plaintext)?;
        drop(plaintext);

        if matches!(payload, EntryPayload::Unknown(_)) {
            // Cannot re-encrypt unknown payloads — abort to preserve the vault.
            return Err(VaultError::UnsupportedEntryType(
                "cannot rewrite unknown entry to drop tag".into(),
            ));
        }
        payload.meta_mut().tag_ids.retain(|t| t != tag_id);

        let new_version = existing.version.checked_add(1).ok_or_else(|| {
            VaultError::MalformedPayload("entry version overflow during tag delete".into())
        })?;
        let aad_new = entry_aad(&entry_id, new_version)?;
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

        let idx_entry = IndexEntry::from_payload(&payload, &row);
        session.index.update_entry(idx_entry);
    }

    // Now drop the tag row itself + index entry.
    session.repo.delete_tag(tag_id).await?;
    session.index.remove_tag(tag_id);

    super::create_entry::append_audit(session, AuditAction::TagDeleted, None).await?;
    Ok(())
}
