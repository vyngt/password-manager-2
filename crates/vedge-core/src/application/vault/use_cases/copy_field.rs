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
use crate::domain::vault::entities::AuditAction;
use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::EntryPayload;
use crate::domain::vault::totp;

/// Which field of the decrypted entry to copy or reveal.
///
/// Both `copy_field` and [`reveal_field`](super::reveal_field) dispatch on this
/// single enum through [`extract_field`], so every arm serves both doors. Adding
/// a variant here is only half the job — the value is unreachable until
/// [`extract_field`] gains a matching arm. The
/// `every_sealed_field_has_a_working_selector` guard test (below) is what forces
/// that second half: it walks every sealed payload field (the same enumeration
/// `resolve_secrets` destructures with no `..`) and fails loudly if a selector
/// does not resolve.
#[derive(Debug, Clone)]
pub enum FieldSelector {
    Password,
    Username,
    TotpCode,
    CardNumber,
    Cvv,
    ApiKey,
    EnvVar(String),
    // ---- 5.4.1 (Complete the Reveal Door) ----
    /// Card PIN (optional secret).
    Pin,
    /// SSH private key (multi-line PEM).
    PrivateKey,
    /// SSH key passphrase (optional secret).
    Passphrase,
    /// API secret — **not** `Secret`: the sibling [`ApiKey`](Self::ApiKey) arm
    /// already names this entry type's other credential.
    ApiSecret,
    /// Identity national ID (optional secret).
    NationalId,
    /// One recovery code by zero-based index. Out-of-range is an error, never an
    /// empty string. (The whole list is fetched via `reveal_recovery_codes`.)
    RecoveryCode(u32),
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

    // 2. Decrypt under the session KEK (shared helper).
    let payload = super::refs::decrypt_row_payload(session, &row)?;

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

    // 4. Update accessed_at + audit (side-effects after crypto work is done). A
    //    secret's plaintext was extracted to the clipboard — audit `SecretRevealed`
    //    (slice 5.4), the action that distinguishes extraction from `Viewed`.
    let when = now();
    session.repo.update_accessed_at(&row.id, when).await?;
    super::create_entry::append_audit(session, AuditAction::SecretRevealed, Some(&row.id)).await?;

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
        FieldSelector::Pin => "pin",
        FieldSelector::PrivateKey => "private_key",
        FieldSelector::Passphrase => "passphrase",
        FieldSelector::ApiSecret => "api_secret",
        FieldSelector::NationalId => "national_id",
        FieldSelector::RecoveryCode(_) => "recovery_code",
        FieldSelector::Custom(_) => "custom",
    }
}

/// Extract one secret field's plaintext from a decrypted payload into a
/// zeroizing buffer. Shared by [`copy_field`] (→ clipboard) and
/// [`reveal_field`](super::reveal_field) (→ renderer): one decrypt-and-select
/// code path, two sinks.
pub(super) fn extract_field(
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
        // ---- 5.4.1 (Complete the Reveal Door) — close the write-only gap ----
        (EntryPayload::Card(p), FieldSelector::Pin) => {
            let Some(pin) = p.pin.as_ref() else {
                return Err(VaultError::FieldNotApplicable);
            };
            Ok(Zeroizing::new(pin.expose_secret().to_owned()))
        }
        (EntryPayload::SshKey(p), FieldSelector::PrivateKey) => {
            // Multi-line PEM — the plaintext (newlines included) crosses byte-exact.
            Ok(Zeroizing::new(p.private_key_pem.expose_secret().to_owned()))
        }
        (EntryPayload::SshKey(p), FieldSelector::Passphrase) => {
            let Some(pass) = p.passphrase.as_ref() else {
                return Err(VaultError::FieldNotApplicable);
            };
            Ok(Zeroizing::new(pass.expose_secret().to_owned()))
        }
        (EntryPayload::ApiKey(p), FieldSelector::ApiSecret) => {
            let Some(secret) = p.secret.as_ref() else {
                return Err(VaultError::FieldNotApplicable);
            };
            Ok(Zeroizing::new(secret.expose_secret().to_owned()))
        }
        (EntryPayload::Identity(p), FieldSelector::NationalId) => {
            let Some(nid) = p.national_id.as_ref() else {
                return Err(VaultError::FieldNotApplicable);
            };
            Ok(Zeroizing::new(nid.expose_secret().to_owned()))
        }
        (EntryPayload::Login(p), FieldSelector::RecoveryCode(i)) => {
            // Indexed access — out of range is `FieldNotApplicable`, never an
            // empty string (a blank code is one a user writes down).
            let idx = usize::try_from(*i).map_err(|_| VaultError::FieldNotApplicable)?;
            let code = p
                .recovery_codes
                .get(idx)
                .ok_or(VaultError::FieldNotApplicable)?;
            Ok(Zeroizing::new(code.expose_secret().to_owned()))
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

    use secrecy::{ExposeSecret, SecretString};

    use super::{FieldSelector, extract_field, field_name};
    use crate::domain::vault::errors::VaultError;
    use crate::domain::vault::payloads::{
        ApiKeyPayload, CardPayload, CommonMeta, EntryPayload, EntryType, EnvVar, EnvVarsPayload,
        IdentityPayload, LoginPayload, SshKeyPayload,
    };
    use crate::domain::vault::totp::TotpParams;

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

    // ---- 5.4.1: the reveal-door guard + per-arm coverage --------------------

    fn expose(s: &SecretString) -> String {
        s.expose_secret().to_owned()
    }

    // Test vectors are deliberately short, low-entropy, non-credential-shaped
    // markers (and avoid the literal `-----BEGIN … PRIVATE KEY-----` header) so
    // GitGuardian's Generic-Password / Private-Key detectors do not flag the PR.
    // `hunter2` is the repo's proven-safe password vector.
    fn login_full() -> EntryPayload {
        EntryPayload::Login(LoginPayload {
            meta: CommonMeta::new("gh", EntryType::Login),
            username: "alice".into(),
            password: SecretString::from("hunter2"),
            totp_secret: None,
            totp_params: TotpParams::default(),
            recovery_codes: vec![
                SecretString::from("rc-0"),
                SecretString::from("rc-1"),
                SecretString::from("rc-2"),
            ],
        })
    }

    fn card_full() -> EntryPayload {
        EntryPayload::Card(CardPayload {
            meta: CommonMeta::new("visa", EntryType::Card),
            cardholder_name: "A B".into(),
            number: SecretString::from("num-0000"),
            expiry_month: 4,
            expiry_year: 2031,
            cvv: SecretString::from("cvv-000"),
            pin: Some(SecretString::from("pin-0000")),
        })
    }

    fn ssh_full() -> EntryPayload {
        EntryPayload::SshKey(SshKeyPayload {
            meta: CommonMeta::new("box", EntryType::SshKey),
            private_key_pem: SecretString::from(
                "-----BEGIN-----\nkey-line-1\nkey-line-2\n-----END-----",
            ),
            passphrase: Some(SecretString::from("phrase-xyz")),
            public_key: "ssh-ed25519 AAA".into(),
            fingerprint: "SHA256:xx".into(),
            key_type: "ed25519".into(),
        })
    }

    fn apikey_full() -> EntryPayload {
        EntryPayload::ApiKey(ApiKeyPayload {
            meta: CommonMeta::new("stripe", EntryType::ApiKey),
            key: SecretString::from("apikey-one"),
            secret: Some(SecretString::from("apikey-two")),
            endpoint: None,
            expiry: None,
            key_type: None,
        })
    }

    fn envvars_full() -> EntryPayload {
        EntryPayload::EnvVars(EnvVarsPayload {
            meta: CommonMeta::new("env", EntryType::EnvVars),
            vars: vec![
                EnvVar {
                    key: "A".into(),
                    value: SecretString::from("env-a-val"),
                },
                EnvVar {
                    key: "B".into(),
                    value: SecretString::from("env-b-val"),
                },
            ],
        })
    }

    fn identity_full() -> EntryPayload {
        EntryPayload::Identity(IdentityPayload {
            meta: CommonMeta::new("me", EntryType::Identity),
            first_name: "A".into(),
            last_name: "B".into(),
            email: "a@b.c".into(),
            phone: None,
            address: None,
            date_of_birth: None,
            national_id: Some(SecretString::from("natid-xyz")),
        })
    }

    /// Every sealed secret field of `payload`, paired with the `FieldSelector`
    /// that MUST extract it. Each variant is destructured with **no `..`**, so
    /// adding a field to any payload struct breaks THIS match at compile time —
    /// the same discipline `resolve_secrets` uses. That is what keeps the seal,
    /// the resolver, and the reveal door on one enumeration: a new sealed field
    /// **forces a look here** (the destructure won't compile until it is bound). It
    /// cannot make the field *fully* airtight — a dev could bind the new field `_`
    /// to bypass — but it converts a silent omission into a deliberate one.
    fn sealed_fields(payload: &EntryPayload) -> Vec<(FieldSelector, String)> {
        match payload {
            EntryPayload::Login(LoginPayload {
                meta: _,
                username: _,
                password,
                // TOTP is reachable via `TotpCode` (a generated code, not the
                // seed) + `reveal_totp`; it is not a direct-extract sealed field.
                totp_secret: _,
                totp_params: _,
                recovery_codes,
            }) => {
                let mut out = vec![(FieldSelector::Password, expose(password))];
                for (i, code) in recovery_codes.iter().enumerate() {
                    let idx = u32::try_from(i).expect("recovery-code index fits u32");
                    out.push((FieldSelector::RecoveryCode(idx), expose(code)));
                }
                out
            }
            EntryPayload::Card(CardPayload {
                meta: _,
                cardholder_name: _,
                number,
                expiry_month: _,
                expiry_year: _,
                cvv,
                pin,
            }) => {
                let mut out = vec![
                    (FieldSelector::CardNumber, expose(number)),
                    (FieldSelector::Cvv, expose(cvv)),
                ];
                if let Some(pin) = pin {
                    out.push((FieldSelector::Pin, expose(pin)));
                }
                out
            }
            EntryPayload::SshKey(SshKeyPayload {
                meta: _,
                private_key_pem,
                passphrase,
                public_key: _,
                fingerprint: _,
                key_type: _,
            }) => {
                let mut out = vec![(FieldSelector::PrivateKey, expose(private_key_pem))];
                if let Some(p) = passphrase {
                    out.push((FieldSelector::Passphrase, expose(p)));
                }
                out
            }
            EntryPayload::ApiKey(ApiKeyPayload {
                meta: _,
                key,
                secret,
                endpoint: _,
                expiry: _,
                key_type: _,
            }) => {
                let mut out = vec![(FieldSelector::ApiKey, expose(key))];
                if let Some(s) = secret {
                    out.push((FieldSelector::ApiSecret, expose(s)));
                }
                out
            }
            EntryPayload::EnvVars(EnvVarsPayload { meta: _, vars }) => vars
                .iter()
                .map(|v| (FieldSelector::EnvVar(v.key.clone()), expose(&v.value)))
                .collect(),
            EntryPayload::Identity(IdentityPayload {
                meta: _,
                first_name: _,
                last_name: _,
                email: _,
                phone: _,
                address: _,
                date_of_birth: _,
                national_id,
            }) => national_id
                .as_ref()
                .map(|n| vec![(FieldSelector::NationalId, expose(n))])
                .unwrap_or_default(),
            // No sealed secrets. `Note.content` crosses plainly on `get_entry`
            // (it is the entry's substance, not credential material) — it is NOT
            // write-only, so it needs no selector.
            EntryPayload::Note(_)
            | EntryPayload::Document(_)
            | EntryPayload::Folder(_)
            | EntryPayload::Unknown(_) => Vec::new(),
        }
    }

    /// 🔴 5.4.1 ③ — THE GUARD. Every sealed secret field must be reachable
    /// through a `FieldSelector` arm. This is the deliverable: it forces the
    /// write-only decision to a compile-time look (a new sealed field can't be
    /// added without visiting `sealed_fields`, which enumerates the seal with a
    /// no-`..` match) and then fails loudly, naming the field, if the door doesn't
    /// cover it. Not fully airtight (a `_` binding could bypass the enumeration),
    /// but it turns a silent omission into a deliberate one.
    #[test]
    fn every_sealed_field_has_a_working_selector() {
        let payloads = [
            login_full(),
            card_full(),
            ssh_full(),
            apikey_full(),
            envvars_full(),
            identity_full(),
        ];

        let mut unreachable: Vec<String> = Vec::new();
        for payload in &payloads {
            let fields = sealed_fields(payload);
            assert!(
                !fields.is_empty(),
                "a payload variant with sealed secrets enumerated none — the guard is blind"
            );
            for (selector, expected) in fields {
                match extract_field(payload, &selector, 0) {
                    Ok(got) if *got == expected => {}
                    Ok(got) => unreachable.push(format!(
                        "{}: selector {selector:?} returned {:?}, expected {expected:?}",
                        field_name(&selector),
                        &*got
                    )),
                    Err(e) => unreachable.push(format!(
                        "{}: selector {selector:?} is write-only ({e:?})",
                        field_name(&selector)
                    )),
                }
            }
        }

        assert!(
            unreachable.is_empty(),
            "sealed fields with no working reveal-door arm ({}):\n{}",
            unreachable.len(),
            unreachable.join("\n")
        );
    }

    #[test]
    fn new_arms_extract_their_secrets() {
        assert_eq!(
            &*extract_field(&card_full(), &FieldSelector::Pin, 0).unwrap(),
            "pin-0000"
        );
        assert_eq!(
            &*extract_field(&ssh_full(), &FieldSelector::Passphrase, 0).unwrap(),
            "phrase-xyz"
        );
        assert_eq!(
            &*extract_field(&apikey_full(), &FieldSelector::ApiSecret, 0).unwrap(),
            "apikey-two"
        );
        assert_eq!(
            &*extract_field(&identity_full(), &FieldSelector::NationalId, 0).unwrap(),
            "natid-xyz"
        );
    }

    /// 🔴 ④'s hazard: the SSH private key is multi-line PEM. Reveal must preserve
    /// `-----BEGIN`, the interior newlines, and `-----END` byte-exact.
    #[test]
    fn private_key_round_trips_multiline_pem_byte_exact() {
        // A PEM-shaped multi-line block WITHOUT the literal `PRIVATE KEY` header
        // (GitGuardian would flag that): the property under test is that the
        // `-----BEGIN`/`-----END` markers and the interior newlines survive
        // byte-exact, not the specific key type.
        let pem = "-----BEGIN VEDGE TEST BLOCK-----\nline-a\nline-b\nline-c\n-----END VEDGE TEST BLOCK-----";
        let ssh = EntryPayload::SshKey(SshKeyPayload {
            meta: CommonMeta::new("box", EntryType::SshKey),
            private_key_pem: SecretString::from(pem),
            passphrase: None,
            public_key: "pk".into(),
            fingerprint: "fp".into(),
            key_type: "ed25519".into(),
        });
        let got = extract_field(&ssh, &FieldSelector::PrivateKey, 0).unwrap();
        assert_eq!(
            &*got, pem,
            "PEM newlines / BEGIN / END must survive byte-exact"
        );
    }

    /// 🔴 ② — a recovery code by index; out-of-range ERRORS, never returns "".
    #[test]
    fn recovery_code_by_index_and_out_of_range_errors() {
        let login = login_full(); // three codes
        assert_eq!(
            &*extract_field(&login, &FieldSelector::RecoveryCode(0), 0).unwrap(),
            "rc-0"
        );
        assert_eq!(
            &*extract_field(&login, &FieldSelector::RecoveryCode(2), 0).unwrap(),
            "rc-2"
        );
        assert!(matches!(
            extract_field(&login, &FieldSelector::RecoveryCode(3), 0),
            Err(VaultError::FieldNotApplicable)
        ));
        assert!(matches!(
            extract_field(&login, &FieldSelector::RecoveryCode(99), 0),
            Err(VaultError::FieldNotApplicable)
        ));
    }

    /// An absent optional secret is `FieldNotApplicable`, never an empty string.
    #[test]
    fn optional_secret_absent_is_not_applicable() {
        let card = EntryPayload::Card(CardPayload {
            meta: CommonMeta::new("visa", EntryType::Card),
            cardholder_name: "n".into(),
            number: SecretString::from("num-0000"),
            expiry_month: 1,
            expiry_year: 2030,
            cvv: SecretString::from("cvv-000"),
            pin: None,
        });
        assert!(matches!(
            extract_field(&card, &FieldSelector::Pin, 0),
            Err(VaultError::FieldNotApplicable)
        ));
    }

    /// The reserved `Custom` arm stays reserved.
    #[test]
    fn custom_selector_still_errors() {
        assert!(matches!(
            extract_field(&login_full(), &FieldSelector::Custom("x".into()), 0),
            Err(VaultError::FieldNotApplicable)
        ));
    }
}
