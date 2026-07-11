//! `copy_field` — decrypt one entry on demand, place a single field on the
//! clipboard, and schedule a background clear.
//!
//! ## Secret-handling discipline
//!
//! - The DEK lives only inside the function body, wrapped in `Zeroizing`;
//!   every control-flow exit releases it.
//! - The decrypted `EntryPayload` zeroizes its secret fields (via
//!   `SecretString`) on drop. We never log its contents.
//! - The plaintext string we hand to the clipboard is materialized on the
//!   heap, copied once into `ClipboardProvider::set`, and then zeroized
//!   before control returns. Once the clipboard owns the string, keeping a
//!   second live copy in our process offers no benefit.
//! - The 30-second clear timer runs in a `tokio::spawn`'d task that holds
//!   an `Arc<dyn ClipboardProvider>` — no `&VaultSession` reference, so the
//!   session can lock/drop while the timer is still pending.

use std::sync::Arc;
use std::time::Duration;

use secrecy::ExposeSecret;
use tracing::instrument;
use zeroize::{Zeroize, Zeroizing};

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{EntryId, now};
use crate::domain::vault::aad::entry_aad;
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;
use crate::domain::vault::totp;

/// Which field of the decrypted entry to copy.
#[derive(Debug, Clone)]
pub enum FieldSelector {
    Password,
    Username,
    TotpCode,
    CardNumber,
    Cvv,
    ApiKey,
    EnvVar(String),
    /// Reserved for future extensibility — always errors today.
    Custom(String),
}

#[derive(Debug)]
pub struct CopyFieldInput {
    pub entry_id: EntryId,
    pub field: FieldSelector,
    /// Seconds to wait before the background task clears the clipboard.
    /// `0` = fire the clear immediately (tests).
    pub clear_after_secs: u32,
}

const DEFAULT_CLEAR_SECS: u32 = 30;

impl CopyFieldInput {
    #[must_use]
    pub const fn new(entry_id: EntryId, field: FieldSelector) -> Self {
        Self {
            entry_id,
            field,
            clear_after_secs: DEFAULT_CLEAR_SECS,
        }
    }
}

#[instrument(skip_all, fields(entry_id = %input.entry_id, field = ?field_name(&input.field)))]
pub async fn copy_field(
    session: &mut VaultSession,
    input: CopyFieldInput,
    now_unix: u64,
) -> Result<(), VaultError> {
    // 1. Fetch ciphertext row — the index doesn't hold secrets.
    let row = session.repo.get_entry(&input.entry_id).await?;

    // 2. Decrypt under the session KEK.
    let dek = session
        .crypto
        .unwrap_dek(&row.dek_wrapped, session.kek.expose())?;
    let aad = entry_aad(&row.id, row.version)?;
    let plaintext = session
        .crypto
        .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)?;
    let payload = EntryPayload::from_decrypted_json(&plaintext)?;

    // 3. Extract the requested field, place it on the clipboard (zeroizing our
    //    copy), and schedule the background clear.
    place_field_on_clipboard(
        session,
        &payload,
        &input.field,
        input.clear_after_secs,
        now_unix,
    )?;

    // Drop decrypted secrets before auditing.
    drop(payload);
    drop(plaintext);
    drop(dek);

    // 4. Update accessed_at + audit (side-effects after crypto work is done).
    let when = now();
    session.repo.update_accessed_at(&row.id, when).await?;
    super::create_entry::append_audit(session, AuditAction::Viewed, Some(&row.id)).await?;

    if let Some(entry) = session.index.entries.get(&row.id).cloned() {
        let mut updated = entry;
        updated.accessed_at = Some(when);
        session.index.update_entry(updated);
    }

    Ok(())
}

/// Extract `field` from an already-decrypted payload, place it on the clipboard,
/// zeroize the local copy, and spawn the detached background clear timer.
///
/// Shared by [`copy_field`] (the live entry) and `copy_history_field` (a prior
/// snapshot) so a historical copy upholds the same discipline: the plaintext is
/// materialized once, handed to the OS clipboard, and zeroized here — it never
/// crosses back to the caller / WASM.
pub(super) fn place_field_on_clipboard(
    session: &VaultSession,
    payload: &EntryPayload,
    field: &FieldSelector,
    clear_after_secs: u32,
    now: u64,
) -> Result<(), VaultError> {
    let value = extract_field(payload, field, now)?;
    place_text_on_clipboard(&session.clipboard, value, clear_after_secs)
}

/// Place a secret on the OS clipboard and schedule its background clear.
///
/// Writes `text` through the injected [`ClipboardProvider`] (which applies the
/// platform exclusion hints), zeroizes our copy, then spawns a detached task
/// that clears the clipboard after `clear_after_secs`.
///
/// Shared by [`copy_field`] / `copy_history_field` (decrypted entry fields) and
/// the generic `copy_text` command (renderer-generated secrets — the password
/// generator and, later, bulk-generate). The clear task holds only an
/// `Arc<dyn ClipboardProvider>` — no `&VaultSession` — so it survives the user
/// locking/dropping the session. **Requires an ambient Tokio runtime** (the
/// Tauri command context and `#[tokio::test]` both provide one).
pub fn place_text_on_clipboard(
    clipboard: &Arc<dyn ClipboardProvider>,
    mut text: Zeroizing<String>,
    clear_after_secs: u32,
) -> Result<(), VaultError> {
    clipboard.set(&text)?;
    text.zeroize();

    // Detached — outlives this session if the user locks. `Arc` is cheap.
    let clipboard: Arc<dyn ClipboardProvider> = Arc::clone(clipboard);
    let delay = Duration::from_secs(u64::from(clear_after_secs));
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        if let Err(e) = clipboard.clear() {
            tracing::warn!(
                error = ?std::mem::discriminant(&e),
                "clipboard clear timer failed"
            );
        }
    });

    Ok(())
}

const fn field_name(f: &FieldSelector) -> &'static str {
    match f {
        FieldSelector::Password => "password",
        FieldSelector::Username => "username",
        FieldSelector::TotpCode => "totp_code",
        FieldSelector::CardNumber => "card_number",
        FieldSelector::Cvv => "cvv",
        FieldSelector::ApiKey => "api_key",
        FieldSelector::EnvVar(_) => "env_var",
        FieldSelector::Custom(_) => "custom",
    }
}

fn extract_field(
    payload: &EntryPayload,
    field: &FieldSelector,
    now: u64,
) -> Result<Zeroizing<String>, VaultError> {
    match (payload, field) {
        (EntryPayload::Login(p), FieldSelector::Username) => Ok(Zeroizing::new(p.username.clone())),
        (EntryPayload::Login(p), FieldSelector::Password) => {
            Ok(Zeroizing::new(p.password.expose_secret().to_owned()))
        }
        (EntryPayload::Login(p), FieldSelector::TotpCode) => {
            let Some(secret) = p.totp_secret.as_ref() else {
                return Err(VaultError::FieldNotApplicable);
            };
            // Same engine + injected clock as `reveal_totp` — one code path.
            Ok(totp::generate(secret, p.totp_params, now)?.code)
        }
        (EntryPayload::Card(p), FieldSelector::CardNumber) => {
            Ok(Zeroizing::new(p.number.expose_secret().to_owned()))
        }
        (EntryPayload::Card(p), FieldSelector::Cvv) => {
            Ok(Zeroizing::new(p.cvv.expose_secret().to_owned()))
        }
        (EntryPayload::ApiKey(p), FieldSelector::ApiKey) => {
            Ok(Zeroizing::new(p.key.expose_secret().to_owned()))
        }
        (EntryPayload::EnvVars(p), FieldSelector::EnvVar(name)) => {
            let found = p
                .vars
                .iter()
                .find(|v| &v.key == name)
                .ok_or(VaultError::FieldNotApplicable)?;
            Ok(Zeroizing::new(found.value.expose_secret().to_owned()))
        }
        _ => Err(VaultError::FieldNotApplicable),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::clone_on_ref_ptr)]

    use std::sync::Arc;
    use std::time::Duration;

    use zeroize::Zeroizing;

    use super::place_text_on_clipboard;
    use crate::application::vault::ports::clipboard::ClipboardProvider;
    use crate::infrastructure::clipboard::MemoryClipboardProvider;

    /// The generic copy path writes the secret through the injected
    /// `ClipboardProvider` (the same port `ArboardClipboardProvider` hardens),
    /// not any renderer/browser path. A far-future clear delay keeps the value
    /// live for the assertion.
    #[tokio::test]
    async fn place_text_on_clipboard_routes_through_provider() {
        let cb = Arc::new(MemoryClipboardProvider::new());
        let provider: Arc<dyn ClipboardProvider> = cb.clone();

        place_text_on_clipboard(&provider, Zeroizing::new("s3cr3t-value".to_owned()), 3600)
            .unwrap();

        assert_eq!(cb.peek().as_deref(), Some("s3cr3t-value"));
        assert_eq!(
            cb.set_count(),
            1,
            "set routed through the provider exactly once"
        );
    }

    /// The detached background clear task still schedules for the generic path.
    /// `clear_after_secs = 0` fires it almost immediately.
    #[tokio::test]
    async fn place_text_on_clipboard_schedules_clear() {
        let cb = Arc::new(MemoryClipboardProvider::new());
        let provider: Arc<dyn ClipboardProvider> = cb.clone();

        place_text_on_clipboard(&provider, Zeroizing::new("ephemeral".to_owned()), 0).unwrap();
        assert_eq!(cb.set_count(), 1);

        // Yield to the spawned clear task (real timer, ~0 s delay).
        for _ in 0..50 {
            if cb.peek().is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(cb.peek(), None, "the clear timer should have fired");
    }
}
