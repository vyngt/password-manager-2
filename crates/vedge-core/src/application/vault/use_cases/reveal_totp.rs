//! `reveal_totp` — decrypt one Login entry, generate the current TOTP code, and
//! return **only** the derivative. The seed never leaves core (the 4.2 door).
//!
//! ## Egress contract
//!
//! The seed flows WASM → core on enrolment only; it never flows core → WASM. The
//! 6–8-digit code is a permitted derivative: short-lived (≤ one period),
//! non-invertible to the seed, worthless once expired.
//!
//! ## Audit — once per (session, entry)
//!
//! Rust decides "first", never WASM. The first reveal emits `TotpRevealed`;
//! period-boundary refreshes in the same session re-generate **audit-silently**
//! (a per-refresh row would be ~2/min of spam and would drown 4.1's page). The
//! dedup set lives on `VaultSession`, so lock → `Drop` clears it and the next
//! unlock audits again. Deliberately does **not** bump `accessed_at` — a code
//! refresh is not an access (`copy_field` is the audited access).

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::EntryId;
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;
use crate::domain::vault::totp::{self, TotpCode};

#[instrument(skip_all, fields(entry_id = %entry_id))]
pub async fn reveal_totp(
    session: &mut VaultSession,
    entry_id: &EntryId,
    now: u64,
) -> Result<TotpCode, VaultError> {
    // Decrypt the one entry (no `accessed_at` bump — this is not an access).
    let row = session.repo.get_entry(entry_id).await?;
    let dek = session
        .crypto
        .unwrap_dek(&row.dek_wrapped, session.kek.expose())?;
    let aad = entry_aad(&row.id, row.version)?;
    let plaintext = session
        .crypto
        .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)?;
    let payload = EntryPayload::from_decrypted_json(&plaintext)?;
    drop(plaintext);
    drop(dek);

    // Generate the code, then drop the payload (and its seed) before auditing.
    let code = {
        let EntryPayload::Login(login) = &payload else {
            return Err(VaultError::FieldNotApplicable);
        };
        let Some(secret) = login.totp_secret.as_ref() else {
            return Err(VaultError::FieldNotApplicable);
        };
        totp::generate(secret, login.totp_params, now)?
    };
    drop(payload);

    // Audit once per (session, entry) — audit *before* marking, so a failed audit
    // write leaves the entry un-marked and re-attempts on the next reveal.
    if !session.revealed_totp.contains(entry_id) {
        super::create_entry::append_audit(session, AuditAction::TotpRevealed, Some(entry_id))
            .await?;
        session.revealed_totp.insert(entry_id.clone());
    }

    Ok(code)
}
