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
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::{AuditAction, EntryHistoryRow};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::health;
use crate::domain::vault::index::IndexEntry;
use crate::domain::vault::payloads::EntryPayload;
use crate::domain::vault::totp::TotpUpdate;

use super::entry_history::HISTORY_MAX_VERSIONS;

pub struct UpdateEntryInput {
    pub entry_id: EntryId,
    /// Complete replacement payload. Callers own the merge logic (read the
    /// current payload, mutate, pass back).
    pub payload: EntryPayload,
    /// TOTP enrolment intent (Login only). Since the seed no longer crosses to
    /// WASM (the 4.2 door), `Unchanged` (the default) carries the stored seed
    /// forward — resolved here, the one place with access to decrypt it.
    pub totp: TotpUpdate,
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

    // Decrypt the existing payload ONCE (reusing the DEK we just unwrapped),
    // unconditionally. Two callers need it: carrying forward an unchanged TOTP
    // seed (the seed no longer crosses to WASM — the 4.2 door), and deciding
    // whether `secret_changed_at` should bump (4.3). Non-Login updates now pay
    // one decrypt+parse; the DEK unwrap was already unconditional.
    let old = super::refs::decrypt_row_with_dek(session.crypto.as_ref(), &dek, &existing)?;
    let when = now();

    // Resolve the TOTP enrolment intent for Login payloads. `Set`/`Clear`
    // enrol/remove; `Unchanged` carries the stored seed forward from `old`.
    let mut new_payload = input.payload;
    if let EntryPayload::Login(login) = &mut new_payload {
        match input.totp {
            TotpUpdate::Set(secret) => login.totp_secret = Some(secret),
            TotpUpdate::Clear => login.totp_secret = None,
            TotpUpdate::Unchanged => {
                if let EntryPayload::Login(old_login) = &old {
                    login.totp_secret.clone_from(&old_login.totp_secret);
                }
            }
        }
    }

    // Stamp secret age: bump only when a secret-bearing field actually changed;
    // otherwise carry the prior stamp forward, so editing a non-secret field
    // does not reset the entry's apparent secret age.
    let secret_stamp = if health::secrets_differ(&old, &new_payload) {
        Some(when)
    } else {
        old.meta().secret_changed_at
    };
    new_payload.meta_mut().secret_changed_at = secret_stamp;
    drop(old);

    let payload_bytes = new_payload.to_encryptable_json()?;

    let aad = entry_aad(&input.entry_id, new_version)?;
    let (nonce, ciphertext) = session.crypto.encrypt_entry(&dek, &payload_bytes, &aad)?;
    // `dek` drops here, zeroizing. The wrapped DEK on disk is unchanged.

    // Snapshot the version we're about to overwrite — a pure byte-copy of the
    // existing ciphertext (no decrypt), keyed to the live entry's stable DEK.
    // Then prune to the retention cap. Only content edits (this use case) create
    // history; metadata-only mutations (favorite/tags/sort/move) deliberately do
    // not, so the log stays a record of real changes.
    session
        .repo
        .insert_history(&EntryHistoryRow::from_superseded(&existing))
        .await?;
    session
        .repo
        .prune_history_keep(&input.entry_id, HISTORY_MAX_VERSIONS)
        .await?;

    let mut row = existing;
    row.version = new_version;
    row.nonce = nonce;
    row.ciphertext = ciphertext;
    row.updated_at = when;
    session.repo.update_entry(&row).await?;

    super::create_entry::append_audit(session, AuditAction::Updated, Some(&input.entry_id)).await?;

    let index_entry = IndexEntry::from_payload(&new_payload, &row);
    session.index.update_entry(index_entry);
    // A content edit re-arms the TOTP reveal audit: a re-enrolled seed is a
    // distinct secret exposure and should audit again this session (slice 4.2).
    session.revealed_totp.remove(&input.entry_id);
    Ok(())
}
