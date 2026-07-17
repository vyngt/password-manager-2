//! `reveal_field` — decrypt one entry on demand and return a single secret
//! field's plaintext to the caller (the renderer), audited as `SecretRevealed`.
//!
//! The audited twin of [`copy_field`](super::copy_field): **same
//! [`FieldSelector`], same decrypt path, same `extract_field`.** The only
//! difference is the sink — `copy_field` hands the plaintext to the OS clipboard
//! and never returns it; `reveal_field` returns it, so it crosses to WASM on an
//! explicit, audited click (slice 5.4's "rarely and audibly").
//!
//! Unlike [`reveal_totp`](super::reveal_totp), there is **no per-session dedup**:
//! a TOTP code auto-refreshes every 30 s (auditing each tick would be noise),
//! but a field reveal is always a deliberate act — every one is logged, exactly
//! as `copy_field` logs every copy.

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;

use super::copy_field::FieldSelector;

#[derive(Debug)]
pub struct RevealFieldInput {
    pub entry_id: EntryId,
    pub field: FieldSelector,
}

impl RevealFieldInput {
    #[must_use]
    pub const fn new(entry_id: EntryId, field: FieldSelector) -> Self {
        Self { entry_id, field }
    }
}

#[instrument(skip_all, fields(entry_id = %input.entry_id))]
pub async fn reveal_field(
    session: &mut VaultSession,
    input: RevealFieldInput,
    now_unix: u64,
) -> Result<Zeroizing<String>, VaultError> {
    // 1. Fetch ciphertext + decrypt under the session KEK — same as copy_field.
    let row = session.repo.get_entry(&input.entry_id).await?;
    let payload = super::refs::decrypt_row_payload(session, &row)?;

    // 2. Select the requested field. The value crosses back to the caller.
    let value = super::copy_field::extract_field(&payload, &input.field, now_unix)?;
    drop(payload);

    // 3. Side-effects mirror copy_field (a reveal is an access): bump
    //    accessed_at, then audit `SecretRevealed` UNCONDITIONALLY — no dedup.
    let when = now();
    session.repo.update_accessed_at(&row.id, when).await?;
    super::create_entry::append_audit(session, AuditAction::SecretRevealed, Some(&row.id)).await?;

    if let Some(entry) = session.index.entries.get(&row.id).cloned() {
        let mut updated = entry;
        updated.accessed_at = Some(when);
        session.index.update_entry(updated);
    }

    Ok(value)
}
