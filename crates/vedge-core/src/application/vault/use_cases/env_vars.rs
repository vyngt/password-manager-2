//! Env-var **set** copy / reveal (slice 5.4.1 ⑥).
//!
//! Sealing env-var values (5.4) removed the way to read the whole set back — but
//! you paste env vars into a `.env` file or JSON config, not one at a time. These
//! two use cases format an `EnvVars` entry's entire set (via
//! [`format_env_vars`](crate::domain::vault::env_export::format_env_vars)) and:
//!
//! - `copy_env_vars` formats **server-side** and places the blob on the OS
//!   clipboard through the hardened [`place_text_on_clipboard`] path — the
//!   plaintext never crosses back to WASM (that would defeat the seal).
//! - `reveal_env_vars` returns the formatted blob to the renderer (like
//!   `reveal_field`), by explicit design.
//!
//! Both audit a **single** `SecretRevealed` row per call (one deliberate act,
//! not one per variable) and bump `accessed_at` (the live entry). The
//! per-variable clipboard copy stays on the `FieldSelector::EnvVar(key)` door.

use tracing::instrument;
use zeroize::Zeroizing;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::env_export::{EnvExportFormat, format_env_vars};
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;

#[derive(Debug)]
pub struct CopyEnvVarsInput {
    pub entry_id: EntryId,
    pub format: EnvExportFormat,
    /// Seconds before the background task clears the clipboard.
    pub clear_after_secs: u32,
}

#[derive(Debug)]
pub struct RevealEnvVarsInput {
    pub entry_id: EntryId,
    pub format: EnvExportFormat,
}

/// Copy the whole env-var set to the OS clipboard (server-side format → hardened
/// clipboard path). One `SecretRevealed` audit row.
#[instrument(skip_all, fields(entry_id = %input.entry_id))]
pub async fn copy_env_vars(
    session: &mut VaultSession,
    input: CopyEnvVarsInput,
) -> Result<(), VaultError> {
    let row = session.repo.get_entry(&input.entry_id).await?;
    let payload = super::refs::decrypt_row_payload(session, &row)?;
    let EntryPayload::EnvVars(env) = &payload else {
        return Err(VaultError::FieldNotApplicable);
    };
    // Format BEFORE any side-effect so a `.env` newline rejection audits nothing.
    let text = format_env_vars(&env.vars, input.format)?;
    drop(payload);

    super::copy_field::place_text_on_clipboard(&session.clipboard, text, input.clear_after_secs)?;

    audit_set_reveal(session, &row.id).await
}

/// Reveal the whole env-var set to the renderer. One `SecretRevealed` audit row.
#[instrument(skip_all, fields(entry_id = %input.entry_id))]
pub async fn reveal_env_vars(
    session: &mut VaultSession,
    input: RevealEnvVarsInput,
) -> Result<Zeroizing<String>, VaultError> {
    let row = session.repo.get_entry(&input.entry_id).await?;
    let payload = super::refs::decrypt_row_payload(session, &row)?;
    let EntryPayload::EnvVars(env) = &payload else {
        return Err(VaultError::FieldNotApplicable);
    };
    let text = format_env_vars(&env.vars, input.format)?;
    drop(payload);

    audit_set_reveal(session, &row.id).await?;
    Ok(text)
}

/// Bump `accessed_at` + append ONE `SecretRevealed` row (the set was extracted).
async fn audit_set_reveal(session: &mut VaultSession, id: &EntryId) -> Result<(), VaultError> {
    let when = now();
    session.repo.update_accessed_at(id, when).await?;
    super::create_entry::append_audit(session, AuditAction::SecretRevealed, Some(id)).await?;
    if let Some(entry) = session.index.entries.get(id).cloned() {
        let mut updated = entry;
        updated.accessed_at = Some(when);
        session.index.update_entry(updated);
    }
    Ok(())
}
