//! Sealed-secret update intents + the resolver (slice 5.4 — the WASM-Secret
//! Sentinel).
//!
//! After the outbound door is sealed, `get_entry` no longer ships a secret
//! field's plaintext, so the inbound (create/update) payload carries an
//! **intent** per sealed field instead of a value. The intent is resolved
//! against the decrypted OLD entry (or `None` on create) by [`resolve_secrets`]
//! — the single place where `Set`/`Clear`/`Unchanged` become real values.
//!
//! Generalizes the 4.2 [`TotpUpdate`] sentinel to every credential field. The
//! whole safety property is `Unchanged` = "carry the stored secret forward":
//! **a dropped field preserves the secret; it never wipes it.**
//!
//! Correctness is structural: [`SecretUpdates`] is a per-payload-variant enum,
//! and [`resolve_secrets`] destructures each variant with **no `..`** — adding a
//! sealed field to a payload forces a field here, and forces it to be handled.
//! A skipped resolution leaves the built payload's placeholder (empty) secret,
//! which any round-trip test catches loudly, instead of silently wiping one
//! field.

use std::collections::HashSet;

use secrecy::SecretString;

use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::{EntryPayload, EnvVar};
use crate::domain::vault::totp::TotpUpdate;

/// Scalar sealed-secret intent (domain side of `SecretUpdateDto`).
#[derive(Debug, Clone, Default)]
pub enum SecretUpdate {
    #[default]
    Unchanged,
    Set(SecretString),
    Clear,
}

/// Ordered-list sealed-secret intent (`Login.recovery_codes`).
#[derive(Debug, Clone, Default)]
pub enum SecretListUpdate {
    #[default]
    Unchanged,
    Set(Vec<SecretString>),
    Clear,
}

/// One keyed row's intent (`EnvVars`). Removal is expressed by omitting the row.
#[derive(Debug, Clone)]
pub struct EnvVarUpdate {
    pub key: String,
    pub value: SecretUpdate,
}

/// Per-payload-variant bundle of sealed-secret intents, parallel to
/// [`EntryPayload`]. Built alongside the placeholder payload in
/// `payload_from_dto`; consumed by [`resolve_secrets`].
#[derive(Debug, Clone)]
pub enum SecretUpdates {
    Login {
        password: SecretUpdate,
        recovery_codes: SecretListUpdate,
        /// TOTP keeps its own sentinel/validation — folded in here so the whole
        /// resolution happens in one place.
        totp: TotpUpdate,
    },
    Card {
        number: SecretUpdate,
        cvv: SecretUpdate,
        pin: SecretUpdate,
    },
    SshKey {
        private_key_pem: SecretUpdate,
        passphrase: SecretUpdate,
    },
    ApiKey {
        key: SecretUpdate,
        secret: SecretUpdate,
    },
    EnvVars {
        vars: Vec<EnvVarUpdate>,
    },
    Identity {
        national_id: SecretUpdate,
    },
    /// No sealed secrets in these variants.
    Note,
    Document,
    Folder,
}

impl SecretUpdates {
    /// Build intents that `Set` every sealed field to the payload's **current**
    /// value. For internal callers that already hold a fully-decrypted payload
    /// (import, restore-from-history, tests) rather than a placeholder payload
    /// built from a sealed WASM DTO — the plaintext is theirs to write directly.
    ///
    /// Optional fields with no value become `Clear` (⇒ `None`), so the resulting
    /// entry mirrors the source payload exactly.
    #[must_use]
    pub fn set_all(payload: &EntryPayload) -> Self {
        match payload {
            EntryPayload::Login(p) => Self::Login {
                password: SecretUpdate::Set(p.password.clone()),
                recovery_codes: SecretListUpdate::Set(p.recovery_codes.clone()),
                totp: p
                    .totp_secret
                    .as_ref()
                    .map_or(TotpUpdate::Clear, |s| TotpUpdate::Set(s.clone())),
            },
            EntryPayload::Card(p) => Self::Card {
                number: SecretUpdate::Set(p.number.clone()),
                cvv: SecretUpdate::Set(p.cvv.clone()),
                pin: opt_set(p.pin.as_ref()),
            },
            EntryPayload::SshKey(p) => Self::SshKey {
                private_key_pem: SecretUpdate::Set(p.private_key_pem.clone()),
                passphrase: opt_set(p.passphrase.as_ref()),
            },
            EntryPayload::ApiKey(p) => Self::ApiKey {
                key: SecretUpdate::Set(p.key.clone()),
                secret: opt_set(p.secret.as_ref()),
            },
            EntryPayload::EnvVars(p) => Self::EnvVars {
                vars: p
                    .vars
                    .iter()
                    .map(|v| EnvVarUpdate {
                        key: v.key.clone(),
                        value: SecretUpdate::Set(v.value.clone()),
                    })
                    .collect(),
            },
            EntryPayload::Identity(p) => Self::Identity {
                national_id: opt_set(p.national_id.as_ref()),
            },
            EntryPayload::Note(_) => Self::Note,
            EntryPayload::Document(_) => Self::Document,
            // Unknown is rejected by create/update before the intents are used;
            // fold it into the harmless Folder marker so this stays total.
            EntryPayload::Folder(_) | EntryPayload::Unknown(_) => Self::Folder,
        }
    }
}

fn opt_set(old: Option<&SecretString>) -> SecretUpdate {
    old.map_or(SecretUpdate::Clear, |s| SecretUpdate::Set(s.clone()))
}

/// Resolve a required scalar: `Set` takes the value, `Unchanged` carries the old
/// one, `Clear` (or `Unchanged` with no old) is illegal — a required credential
/// cannot be emptied.
fn resolve_required(
    intent: SecretUpdate,
    old: Option<&SecretString>,
    field: &'static str,
) -> Result<SecretString, VaultError> {
    match intent {
        SecretUpdate::Set(s) => Ok(s),
        SecretUpdate::Unchanged => old.cloned().ok_or(VaultError::RequiredSecretCleared(field)),
        SecretUpdate::Clear => Err(VaultError::RequiredSecretCleared(field)),
    }
}

/// Resolve an optional scalar: `Set` = value, `Clear` = none, `Unchanged` =
/// carry the old (which may itself be none).
fn resolve_optional(intent: SecretUpdate, old: Option<&SecretString>) -> Option<SecretString> {
    match intent {
        SecretUpdate::Set(s) => Some(s),
        SecretUpdate::Clear => None,
        SecretUpdate::Unchanged => old.cloned(),
    }
}

/// Resolve an ordered list: `Set` replaces, `Clear` empties, `Unchanged` carries
/// the old list forward.
fn resolve_list(intent: SecretListUpdate, old: &[SecretString]) -> Vec<SecretString> {
    match intent {
        SecretListUpdate::Set(v) => v,
        SecretListUpdate::Clear => Vec::new(),
        SecretListUpdate::Unchanged => old.to_vec(),
    }
}

/// Resolve a keyed collection by matching each inbound row against the old rows
/// by key. Rows omitted from `rows` are dropped (that is how removal works under
/// the complete-replacement contract). Duplicate keys, a `Clear` on a row, and an
/// `Unchanged` for a brand-new key are all hard errors.
fn resolve_keyed(rows: Vec<EnvVarUpdate>, old: &[EnvVar]) -> Result<Vec<EnvVar>, VaultError> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if !seen.insert(row.key.clone()) {
            return Err(VaultError::DuplicateEnvVarKey(row.key));
        }
        let value = match row.value {
            SecretUpdate::Set(v) => v,
            SecretUpdate::Unchanged => old
                .iter()
                .find(|o| o.key == row.key)
                .map(|o| o.value.clone())
                .ok_or_else(|| VaultError::EnvVarUnchangedWithoutStored(row.key.clone()))?,
            SecretUpdate::Clear => {
                return Err(VaultError::MalformedPayload(format!(
                    "env var '{}' cannot be Clear; omit the row to remove it",
                    row.key
                )));
            }
        };
        out.push(EnvVar {
            key: row.key,
            value,
        });
    }
    Ok(out)
}

/// Fill `new`'s sealed-secret fields from the intents.
///
/// Carries forward from the decrypted `old` entry (`None` on create); called by
/// `create_entry` and `update_entry` before `secrets_differ` and re-encryption.
/// The exhaustive `match` with no `..` is the anti-silent-wipe guard: a new
/// sealed field cannot be added without handling it here.
pub fn resolve_secrets(
    new: &mut EntryPayload,
    intents: SecretUpdates,
    old: Option<&EntryPayload>,
) -> Result<(), VaultError> {
    match (new, intents) {
        (
            EntryPayload::Login(p),
            SecretUpdates::Login {
                password,
                recovery_codes,
                totp,
            },
        ) => {
            let o = match old {
                Some(EntryPayload::Login(l)) => Some(l),
                _ => None,
            };
            p.password = resolve_required(password, o.map(|l| &l.password), "login.password")?;
            p.recovery_codes = resolve_list(
                recovery_codes,
                o.map_or(&[][..], |l| l.recovery_codes.as_slice()),
            );
            p.totp_secret = match totp {
                TotpUpdate::Set(s) => Some(s),
                TotpUpdate::Clear => None,
                TotpUpdate::Unchanged => o.and_then(|l| l.totp_secret.clone()),
            };
        }
        (EntryPayload::Card(p), SecretUpdates::Card { number, cvv, pin }) => {
            let o = match old {
                Some(EntryPayload::Card(c)) => Some(c),
                _ => None,
            };
            p.number = resolve_required(number, o.map(|c| &c.number), "card.number")?;
            p.cvv = resolve_required(cvv, o.map(|c| &c.cvv), "card.cvv")?;
            p.pin = resolve_optional(pin, o.and_then(|c| c.pin.as_ref()));
        }
        (
            EntryPayload::SshKey(p),
            SecretUpdates::SshKey {
                private_key_pem,
                passphrase,
            },
        ) => {
            let o = match old {
                Some(EntryPayload::SshKey(s)) => Some(s),
                _ => None,
            };
            p.private_key_pem = resolve_required(
                private_key_pem,
                o.map(|s| &s.private_key_pem),
                "ssh.private_key_pem",
            )?;
            p.passphrase = resolve_optional(passphrase, o.and_then(|s| s.passphrase.as_ref()));
        }
        (EntryPayload::ApiKey(p), SecretUpdates::ApiKey { key, secret }) => {
            let o = match old {
                Some(EntryPayload::ApiKey(a)) => Some(a),
                _ => None,
            };
            p.key = resolve_required(key, o.map(|a| &a.key), "apikey.key")?;
            p.secret = resolve_optional(secret, o.and_then(|a| a.secret.as_ref()));
        }
        (EntryPayload::EnvVars(p), SecretUpdates::EnvVars { vars }) => {
            let o = match old {
                Some(EntryPayload::EnvVars(e)) => e.vars.as_slice(),
                _ => &[][..],
            };
            p.vars = resolve_keyed(vars, o)?;
        }
        (EntryPayload::Identity(p), SecretUpdates::Identity { national_id }) => {
            let o = match old {
                Some(EntryPayload::Identity(i)) => Some(i),
                _ => None,
            };
            p.national_id = resolve_optional(national_id, o.and_then(|i| i.national_id.as_ref()));
        }
        (EntryPayload::Note(_), SecretUpdates::Note)
        | (EntryPayload::Document(_), SecretUpdates::Document)
        | (EntryPayload::Folder(_), SecretUpdates::Folder) => {}
        _ => {
            return Err(VaultError::MalformedPayload(
                "payload/intent variant mismatch".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use super::*;
    use crate::domain::vault::payloads::{
        CardPayload, CommonMeta, EntryType, EnvVarsPayload, LoginPayload, NotePayload,
    };
    use crate::domain::vault::totp::TotpParams;
    use secrecy::ExposeSecret;

    fn login(pw: &str, codes: &[&str]) -> EntryPayload {
        EntryPayload::Login(LoginPayload {
            meta: CommonMeta::new("demo", EntryType::Login),
            username: "u".into(),
            password: SecretString::from(pw),
            totp_secret: None,
            totp_params: TotpParams::default(),
            recovery_codes: codes.iter().map(|c| SecretString::from(*c)).collect(),
        })
    }

    fn login_intents(password: SecretUpdate, recovery_codes: SecretListUpdate) -> SecretUpdates {
        SecretUpdates::Login {
            password,
            recovery_codes,
            totp: TotpUpdate::Unchanged,
        }
    }

    fn card(number: &str, cvv: &str, pin: Option<&str>) -> EntryPayload {
        EntryPayload::Card(CardPayload {
            meta: CommonMeta::new("c", EntryType::Card),
            cardholder_name: "n".into(),
            number: SecretString::from(number),
            expiry_month: 1,
            expiry_year: 2030,
            cvv: SecretString::from(cvv),
            pin: pin.map(SecretString::from),
        })
    }

    fn empty_envvars() -> EntryPayload {
        EntryPayload::EnvVars(EnvVarsPayload {
            meta: CommonMeta::new("e", EntryType::EnvVars),
            vars: vec![],
        })
    }

    #[test]
    fn unchanged_carries_the_stored_secret_forward() {
        // The safe failure mode: an update that never touched the password (the
        // field defaults to Unchanged) preserves the stored password + codes.
        let old = login("hunter2", &["rc1", "rc2"]);
        let mut new = login("", &[]);
        resolve_secrets(
            &mut new,
            login_intents(SecretUpdate::Unchanged, SecretListUpdate::Unchanged),
            Some(&old),
        )
        .unwrap();
        let EntryPayload::Login(l) = new else {
            panic!()
        };
        assert_eq!(l.password.expose_secret(), "hunter2");
        assert_eq!(l.recovery_codes.len(), 2);
        assert_eq!(l.recovery_codes[0].expose_secret(), "rc1");
    }

    #[test]
    fn set_replaces_the_secret() {
        let old = login("hunter2", &[]);
        let mut new = login("", &[]);
        resolve_secrets(
            &mut new,
            login_intents(
                SecretUpdate::Set(SecretString::from("hunter3")),
                SecretListUpdate::Unchanged,
            ),
            Some(&old),
        )
        .unwrap();
        let EntryPayload::Login(l) = new else {
            panic!()
        };
        assert_eq!(l.password.expose_secret(), "hunter3");
    }

    #[test]
    fn required_secret_rejects_clear() {
        let old = login("pw", &[]);
        let mut new = login("", &[]);
        let err = resolve_secrets(
            &mut new,
            login_intents(SecretUpdate::Clear, SecretListUpdate::Unchanged),
            Some(&old),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            VaultError::RequiredSecretCleared("login.password")
        ));
    }

    #[test]
    fn required_secret_rejects_unchanged_on_create() {
        // Create (old = None) + Unchanged on a required field = nothing to carry.
        let mut new = login("", &[]);
        let err = resolve_secrets(
            &mut new,
            login_intents(SecretUpdate::Unchanged, SecretListUpdate::Unchanged),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            VaultError::RequiredSecretCleared("login.password")
        ));
    }

    #[test]
    fn optional_secret_clear_yields_none() {
        let old = card("4111", "123", Some("9999"));
        let mut new = card("", "", Some("placeholder"));
        resolve_secrets(
            &mut new,
            SecretUpdates::Card {
                number: SecretUpdate::Unchanged,
                cvv: SecretUpdate::Unchanged,
                pin: SecretUpdate::Clear,
            },
            Some(&old),
        )
        .unwrap();
        let EntryPayload::Card(c) = new else { panic!() };
        assert_eq!(c.number.expose_secret(), "4111", "required carried forward");
        assert!(c.pin.is_none(), "optional Clear yields None");
    }

    #[test]
    fn envvars_carry_by_key_replace_and_drop() {
        let old = EntryPayload::EnvVars(EnvVarsPayload {
            meta: CommonMeta::new("e", EntryType::EnvVars),
            vars: vec![
                EnvVar {
                    key: "A".into(),
                    value: SecretString::from("old-a"),
                },
                EnvVar {
                    key: "B".into(),
                    value: SecretString::from("old-b"),
                },
                EnvVar {
                    key: "C".into(),
                    value: SecretString::from("old-c"),
                },
            ],
        });
        // Keep A (Unchanged -> old-a), replace B (Set), drop C by omission.
        let mut new = empty_envvars();
        resolve_secrets(
            &mut new,
            SecretUpdates::EnvVars {
                vars: vec![
                    EnvVarUpdate {
                        key: "A".into(),
                        value: SecretUpdate::Unchanged,
                    },
                    EnvVarUpdate {
                        key: "B".into(),
                        value: SecretUpdate::Set(SecretString::from("new-b")),
                    },
                ],
            },
            Some(&old),
        )
        .unwrap();
        let EntryPayload::EnvVars(e) = new else {
            panic!()
        };
        assert_eq!(e.vars.len(), 2, "C dropped by omission");
        assert_eq!(e.vars[0].value.expose_secret(), "old-a");
        assert_eq!(e.vars[1].value.expose_secret(), "new-b");
    }

    #[test]
    fn envvars_reject_unchanged_new_key_and_duplicates() {
        let mut new = empty_envvars();
        let err = resolve_secrets(
            &mut new,
            SecretUpdates::EnvVars {
                vars: vec![EnvVarUpdate {
                    key: "NEW".into(),
                    value: SecretUpdate::Unchanged,
                }],
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, VaultError::EnvVarUnchangedWithoutStored(_)));

        let mut new2 = empty_envvars();
        let err = resolve_secrets(
            &mut new2,
            SecretUpdates::EnvVars {
                vars: vec![
                    EnvVarUpdate {
                        key: "X".into(),
                        value: SecretUpdate::Set(SecretString::from("1")),
                    },
                    EnvVarUpdate {
                        key: "X".into(),
                        value: SecretUpdate::Set(SecretString::from("2")),
                    },
                ],
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(err, VaultError::DuplicateEnvVarKey(_)));
    }

    #[test]
    fn variant_mismatch_errors() {
        let mut note = EntryPayload::Note(NotePayload {
            meta: CommonMeta::new("n", EntryType::Note),
            content: SecretString::from("body"),
        });
        let err = resolve_secrets(
            &mut note,
            login_intents(
                SecretUpdate::Set(SecretString::from("x")),
                SecretListUpdate::Unchanged,
            ),
            None,
        )
        .unwrap_err();
        assert!(matches!(err, VaultError::MalformedPayload(_)));
    }

    #[test]
    fn set_all_extracts_every_secret() {
        match SecretUpdates::set_all(&login("pw", &["rc"])) {
            SecretUpdates::Login {
                password,
                recovery_codes,
                ..
            } => {
                assert!(matches!(password, SecretUpdate::Set(_)));
                assert!(matches!(recovery_codes, SecretListUpdate::Set(_)));
            }
            _ => panic!("expected Login"),
        }
    }
}
