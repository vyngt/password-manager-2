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

    // 3. Extract the requested field into a zeroizing String.
    let mut value = extract_field(&payload, &input.field)?;

    // 4. Hand to clipboard — then zeroize our copy immediately.
    session.clipboard.set(&value)?;
    value.zeroize();

    // Drop decrypted secrets before auditing / scheduling the timer.
    drop(payload);
    drop(plaintext);
    drop(dek);

    // 5. Update accessed_at + audit (side-effects after crypto work is done).
    let when = now();
    session.repo.update_accessed_at(&row.id, when).await?;
    super::create_entry::append_audit(session, AuditAction::Viewed, Some(&row.id)).await?;

    if let Some(entry) = session.index.entries.get(&row.id).cloned() {
        let mut updated = entry;
        updated.accessed_at = Some(when);
        session.index.update_entry(updated);
    }

    // 6. Spawn the clear timer. Detached — outlives this session if the user
    // locks. `Arc<dyn ClipboardProvider>` is cheap to clone.
    let clipboard: Arc<dyn ClipboardProvider> = Arc::clone(&session.clipboard);
    let delay = Duration::from_secs(u64::from(input.clear_after_secs));
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
            Ok(Zeroizing::new(generate_totp(secret.expose_secret())?))
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

/// RFC 6238 TOTP (SHA-1, 6 digits, 30-second period) — matches the defaults
/// every authenticator app uses. Returns a 6-digit numeric string.
fn generate_totp(seed_b32: &str) -> Result<String, VaultError> {
    let bytes = totp_rs::Secret::Encoded(seed_b32.to_owned())
        .to_bytes()
        .map_err(|_| VaultError::MalformedPayload("invalid TOTP secret encoding".into()))?;
    let totp = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, bytes)
        .map_err(|e| VaultError::MalformedPayload(format!("TOTP init: {e}")))?;
    totp.generate_current()
        .map_err(|e| VaultError::MalformedPayload(format!("TOTP generate: {e}")))
}
