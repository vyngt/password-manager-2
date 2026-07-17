//! Entry version history — list / reveal / restore / copy prior versions.
//!
//! Snapshots are captured (a pure byte-copy of the superseded ciphertext) inside
//! [`update_entry`](super::update_entry) and stored in `entry_history` with **no
//! key material** (see [`EntryHistoryRow`]). Every path here decrypts a snapshot
//! by unwrapping the DEK from the **live** `entries` row and authenticating
//! against the snapshot's *own* version — so history survives a master-password
//! change (`change_password` re-wraps DEKs O(1) without rewriting ciphertext).
//!
//! Secret-handling boundaries:
//! - [`list_history`] decrypts server-side only to diff *field names* — no secret
//!   values leave the backend, so it is **not** audited.
//! - [`get_history_value`] is the sanctioned reveal: it returns the decrypted
//!   payload and audits `Viewed`, exactly like [`get_entry`](super::get_entry).
//! - [`copy_history_field`] routes a prior secret to the OS clipboard without
//!   returning it to the caller (the `copy_field` discipline).
//! - [`restore_from_history`] re-encrypts server-side via `update_entry`; no
//!   plaintext crosses to WASM.

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, Timestamp};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::crypto_constants::{DEK_LEN, NONCE_LEN};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;

use super::copy_field::FieldSelector;

/// Max snapshots retained per entry; the oldest are pruned on capture.
pub const HISTORY_MAX_VERSIONS: usize = 20;

/// One version in an entry's timeline — **metadata only**, no secret values.
#[derive(Debug, Clone)]
pub struct HistoryVersion {
    /// `None` = the current live version; `Some(id)` = a snapshot row.
    pub history_id: Option<String>,
    /// The entry's version at this point in the timeline.
    pub version: i64,
    /// When this version became current.
    pub changed_at: Timestamp,
    /// Field-name keys that differ from the next-older version. Empty for the
    /// origin version. Content fields only (UI-meta excluded).
    pub changed_fields: Vec<String>,
}

/// Input for [`copy_history_field`].
#[derive(Debug)]
pub struct CopyHistoryFieldInput {
    pub entry_id: EntryId,
    pub history_id: String,
    pub field: FieldSelector,
    pub clear_after_secs: u32,
}

/// The whole version timeline for an entry, newest-first, with per-version
/// changed-field summaries. The first element is the current live version
/// (`history_id = None`); the rest are snapshots.
#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn list_history(
    session: &VaultSession,
    entry_id: &EntryId,
) -> Result<Vec<HistoryVersion>, VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }

    let live = session.repo.get_entry(entry_id).await?;
    let snapshots = session.repo.list_history(entry_id).await?;

    // One DEK unwrap from the live row covers every version (stable per-entry key).
    let dek = session
        .crypto
        .unwrap_dek(&live.dek_wrapped, session.kek.expose())?;

    // Timeline newest → oldest: the live version, then snapshots (version-desc).
    // Decrypt each payload to a JSON object so we can diff field *names* between
    // neighbouring versions.
    let mut jsons: Vec<serde_json::Value> = Vec::with_capacity(snapshots.len().saturating_add(1));
    jsons.push(decrypt_json(
        session,
        &dek,
        &live.nonce,
        &live.ciphertext,
        entry_id,
        live.version,
    )?);
    for snap in &snapshots {
        jsons.push(decrypt_json(
            session,
            &dek,
            &snap.nonce,
            &snap.ciphertext,
            entry_id,
            snap.version,
        )?);
    }
    drop(dek);

    // Metadata in the same newest → oldest order as `jsons`.
    let meta = std::iter::once((None, live.version, live.updated_at)).chain(
        snapshots
            .iter()
            .map(|s| (Some(s.id.clone()), s.version, s.changed_at)),
    );
    // The next-older version's JSON for each row (`None` for the origin).
    let older = jsons.iter().skip(1).map(Some).chain(std::iter::once(None));

    let out = meta
        .zip(jsons.iter())
        .zip(older)
        .map(
            |(((history_id, version, changed_at), cur), older)| HistoryVersion {
                history_id,
                version,
                changed_at,
                changed_fields: older.map_or_else(Vec::new, |o| changed_fields(o, cur)),
            },
        )
        .collect();
    Ok(out)
}

/// Reveal a prior version's full decrypted payload.
///
/// The sanctioned, audited (`Viewed`) reveal path, mirroring
/// [`get_entry`](super::get_entry). Secrets stay in `SecretString` (zeroize on
/// drop); the caller owns that lifetime.
#[instrument(skip_all, fields(entry_id = %entry_id, history_id = %history_id))]
pub async fn get_history_value(
    session: &VaultSession,
    entry_id: &EntryId,
    history_id: &str,
) -> Result<EntryPayload, VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }
    let payload = decrypt_snapshot(session, entry_id, history_id).await?;

    if matches!(payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot reveal an unknown entry".into(),
        ));
    }

    // A reveal exposes secrets to the UI — audit it, like `get_entry`. We do not
    // touch `accessed_at`: a prior version was viewed, not the current one.
    super::create_entry::append_audit(session, AuditAction::Viewed, Some(entry_id)).await?;
    Ok(payload)
}

/// Copy a single field of a prior version to the OS clipboard (auto-clear). The
/// plaintext never returns to the caller — same discipline as `copy_field`.
#[instrument(skip_all, fields(entry_id = %input.entry_id, history_id = %input.history_id))]
pub async fn copy_history_field(
    session: &VaultSession,
    input: CopyHistoryFieldInput,
    now: u64,
) -> Result<(), VaultError> {
    if !session.index.entries.contains_key(&input.entry_id) {
        return Err(VaultError::EntryNotFound(input.entry_id.clone()));
    }
    let payload = decrypt_snapshot(session, &input.entry_id, &input.history_id).await?;
    super::copy_field::place_field_on_clipboard(
        session,
        &payload,
        &input.field,
        input.clear_after_secs,
        now,
    )?;
    drop(payload);

    // A secret's plaintext was extracted to the clipboard — audit `SecretRevealed`
    // (slice 5.4), distinguishing extraction from a metadata-only `Viewed`.
    super::create_entry::append_audit(session, AuditAction::SecretRevealed, Some(&input.entry_id))
        .await?;
    Ok(())
}

/// Input for [`reveal_history_field`].
#[derive(Debug)]
pub struct RevealHistoryFieldInput {
    pub entry_id: EntryId,
    pub history_id: String,
    pub field: FieldSelector,
}

/// Reveal a single field of a prior version to the renderer (slice 5.4).
///
/// The history twin of [`reveal_field`](super::reveal_field), and the reveal
/// counterpart of [`copy_history_field`]: decrypt the snapshot, return ONE
/// field's plaintext, and audit `SecretRevealed`. No `accessed_at` bump — a
/// prior version was inspected, not the current one (matching
/// [`get_history_value`]). Audited unconditionally, no dedup.
#[instrument(skip_all, fields(entry_id = %input.entry_id, history_id = %input.history_id))]
pub async fn reveal_history_field(
    session: &VaultSession,
    input: RevealHistoryFieldInput,
    now: u64,
) -> Result<Zeroizing<String>, VaultError> {
    if !session.index.entries.contains_key(&input.entry_id) {
        return Err(VaultError::EntryNotFound(input.entry_id.clone()));
    }
    let payload = decrypt_snapshot(session, &input.entry_id, &input.history_id).await?;
    let value = super::copy_field::extract_field(&payload, &input.field, now)?;
    drop(payload);

    super::create_entry::append_audit(session, AuditAction::SecretRevealed, Some(&input.entry_id))
        .await?;
    Ok(value)
}

/// Set the entry back to a prior version.
///
/// Decrypts the snapshot server-side and routes the payload through
/// [`update_entry`](super::update_entry), which snapshots the now-current value
/// first, bumps the version, and audits. **No plaintext crosses to WASM** — the
/// reveal is not audited here (the value is re-encrypted, not shown).
#[instrument(skip_all, fields(entry_id = %entry_id, history_id = %history_id))]
pub async fn restore_from_history(
    session: &mut VaultSession,
    entry_id: &EntryId,
    history_id: &str,
) -> Result<(), VaultError> {
    if !session.index.entries.contains_key(entry_id) {
        return Err(VaultError::EntryNotFound(entry_id.clone()));
    }
    let payload = decrypt_snapshot(session, entry_id, history_id).await?;

    if matches!(payload, EntryPayload::Unknown(_)) {
        return Err(VaultError::UnsupportedEntryType(
            "cannot restore an unknown entry".into(),
        ));
    }

    // `update_entry` treats `totp` as authoritative (the seed no longer travels
    // via the payload alone). A restore must revert to the *snapshot's* TOTP, not
    // carry the current one forward, so translate the snapshot seed into an
    // explicit `Set`/`Clear` intent.
    // The snapshot payload holds the real (decrypted) secrets — write exactly the
    // snapshot's values (slice 5.4): `full` derives `Set`-everything intents.
    super::update_entry::update_entry(
        session,
        super::update_entry::UpdateEntryInput::full(entry_id.clone(), payload),
    )
    .await
}

/// Field-name keys where `cur` differs from `prev` — the changed-field chips.
///
/// A generic top-level JSON-object diff over the two decrypted payloads. Works
/// for every entry type without per-variant code; a change inside a nested value
/// (e.g. one env var) surfaces as its containing key. Non-content meta is
/// excluded so a favorite/sort/tag/folder change never reads as a changed field.
/// Returns keys sorted for deterministic output.
#[must_use]
pub fn changed_fields(prev: &serde_json::Value, cur: &serde_json::Value) -> Vec<String> {
    const UI_META: &[&str] = &[
        "is_favorite",
        "sort_order",
        "tag_ids",
        "folder_id",
        "payload_schema",
        "entry_type",
        // Derived age stamp owned by the write path (4.3) — bumps *because* a
        // secret changed, so surfacing it would double the real field's chip.
        "secret_changed_at",
    ];
    let empty = serde_json::Map::new();
    let p = prev.as_object().unwrap_or(&empty);
    let c = cur.as_object().unwrap_or(&empty);

    let mut keys: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for k in p.keys().chain(c.keys()) {
        if !UI_META.contains(&k.as_str()) {
            keys.insert(k);
        }
    }
    keys.into_iter()
        .filter(|k| p.get(*k) != c.get(*k))
        .map(str::to_owned)
        .collect()
}

/// Decrypt one snapshot (or the live row) into a JSON object for field diffing.
/// AAD binds the snapshot's *own* version. Values stay server-side.
fn decrypt_json(
    session: &VaultSession,
    dek: &[u8; DEK_LEN],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
    entry_id: &EntryId,
    version: i64,
) -> Result<serde_json::Value, VaultError> {
    let aad = entry_aad(entry_id, version)?;
    let plaintext = session.crypto.decrypt_entry(dek, nonce, ciphertext, &aad)?;
    serde_json::from_slice(&plaintext).map_err(|e| VaultError::MalformedPayload(e.to_string()))
}

/// Decrypt a snapshot into an `EntryPayload`, unwrapping the DEK from the live
/// row and authenticating against the snapshot's version. No audit — callers add
/// the appropriate side-effect.
async fn decrypt_snapshot(
    session: &VaultSession,
    entry_id: &EntryId,
    history_id: &str,
) -> Result<EntryPayload, VaultError> {
    let live = session.repo.get_entry(entry_id).await?;
    let snap = session.repo.get_history(history_id).await?;
    // A snapshot can only be read against its own entry.
    if snap.entry_id != *entry_id {
        return Err(VaultError::HistoryNotFound(history_id.to_owned()));
    }

    let dek = session
        .crypto
        .unwrap_dek(&live.dek_wrapped, session.kek.expose())?;
    let aad = entry_aad(entry_id, snap.version)?;
    let plaintext = session
        .crypto
        .decrypt_entry(&dek, &snap.nonce, &snap.ciphertext, &aad)?;
    EntryPayload::from_decrypted_json(&plaintext)
}

#[cfg(test)]
mod tests {
    use super::changed_fields;
    use serde_json::json;

    #[test]
    fn reports_edited_content_field() {
        let prev = json!({"entry_type": "Login", "name": "GitHub", "password": "p1"});
        let cur = json!({"entry_type": "Login", "name": "GitHub", "password": "p2"});
        assert_eq!(changed_fields(&prev, &cur), vec!["password".to_owned()]);
    }

    #[test]
    fn reports_added_and_removed_fields_sorted() {
        let prev = json!({"name": "x", "cvv": "111"});
        let cur = json!({"name": "x", "number": "4111"});
        // `cvv` removed, `number` added — both differ; sorted output.
        assert_eq!(
            changed_fields(&prev, &cur),
            vec!["cvv".to_owned(), "number".to_owned()]
        );
    }

    #[test]
    fn ignores_ui_meta_and_identical_fields() {
        let prev = json!({
            "entry_type": "Card", "name": "Visa",
            "is_favorite": false, "sort_order": 0, "tag_ids": [], "folder_id": null,
            "payload_schema": 1, "number": "4111"
        });
        let cur = json!({
            "entry_type": "Card", "name": "Visa",
            "is_favorite": true, "sort_order": 9, "tag_ids": ["t1"], "folder_id": "f1",
            "payload_schema": 1, "number": "4111",
            // The 4.3 age stamp is UI-meta: bumping it must not read as a content
            // change (it would double the real field's chip).
            "secret_changed_at": "2026-07-12T00:00:00Z"
        });
        // Only UI-meta changed → no content field reported.
        assert!(changed_fields(&prev, &cur).is_empty());
    }

    #[test]
    fn identical_payloads_report_nothing() {
        let v = json!({"entry_type": "Note", "name": "n", "content": "same"});
        assert!(changed_fields(&v, &v).is_empty());
    }

    #[test]
    fn nested_change_surfaces_containing_key() {
        let prev = json!({"entry_type": "EnvVars", "vars": [{"key": "A", "value": "1"}]});
        let cur = json!({"entry_type": "EnvVars", "vars": [{"key": "A", "value": "2"}]});
        assert_eq!(changed_fields(&prev, &cur), vec!["vars".to_owned()]);
    }
}
