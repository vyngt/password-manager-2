//! Entry DTO conversion layer (wire types from `vedge_ipc`).

pub use vedge_ipc::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, DocumentPayloadDto, EnvVarUpdateDto,
    EnvVarsPayloadDto, FolderPayloadDto, HistoryEntryDto, IdentityPayloadDto, IndexEntryDto,
    LoginPayloadDto, NotePayloadDto, PayloadDto, SshKeyPayloadDto,
};

use secrecy::{ExposeSecret, SecretString};
use vedge_ipc::{SecretListUpdateDto, SecretUpdateDto, TotpUpdateDto};

use crate::dto::secret_update::{
    env_var_update_from_dto, secret_list_update_from_dto, secret_update_from_dto,
};
use crate::dto::totp::{totp_algorithm_to_dto, totp_params_from_dto, totp_update_from_dto};

use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::index::IndexEntry;
use vedge_core::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, DocumentPayload, EntryPayload, EntryType, EnvVarsPayload,
    FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
};
use vedge_core::{HistoryVersion, SecretUpdates};

use crate::dto::common::{
    b64_decode_fixed, b64_encode, common_meta_from_dto, common_meta_to_dto, entry_type_to_dto,
    ts_to_string,
};
use crate::error::CommandError;

// ---- IndexEntry → DTO --------------------------------------------------------

#[must_use]
pub fn index_entry_to_dto(e: &IndexEntry) -> IndexEntryDto {
    IndexEntryDto {
        id: e.id.as_str().to_owned(),
        name: e.name.clone(),
        entry_type: entry_type_to_dto(&e.entry_type),
        url: e.url.clone(),
        favicon_url: e.favicon_url.clone(),
        tag_ids: e.tag_ids.iter().map(|t| t.as_str().to_owned()).collect(),
        folder_id: e.folder_id.as_ref().map(|f| f.as_str().to_owned()),
        is_favorite: e.is_favorite,
        color: e.color.clone(),
        icon: e.icon.clone(),
        sort_order: e.sort_order,
        is_trashed: e.is_trashed,
        cipher_suite: e.cipher_suite,
        created_at: ts_to_string(e.created_at),
        updated_at: ts_to_string(e.updated_at),
        accessed_at: e.accessed_at.map(ts_to_string),
    }
}

// ---- HistoryVersion → DTO ----------------------------------------------------

#[must_use]
pub fn history_to_dto(v: &HistoryVersion) -> HistoryEntryDto {
    HistoryEntryDto {
        history_id: v.history_id.clone(),
        version: u32::try_from(v.version).unwrap_or(u32::MAX),
        changed_at: ts_to_string(v.changed_at),
        changed_fields: v.changed_fields.clone(),
    }
}

// ---- Address -----------------------------------------------------------------

#[must_use]
pub fn address_to_dto(a: &Address) -> AddressDto {
    AddressDto {
        line1: a.line1.clone(),
        line2: a.line2.clone(),
        city: a.city.clone(),
        state: a.state.clone(),
        postal_code: a.postal_code.clone(),
        country: a.country.clone(),
    }
}

#[must_use]
pub fn address_from_dto(a: AddressDto) -> Address {
    Address {
        line1: a.line1,
        line2: a.line2,
        city: a.city,
        state: a.state,
        postal_code: a.postal_code,
        country: a.country,
    }
}

// ---- Payload round-trip ------------------------------------------------------

/// Convert a write-side DTO into a domain `EntryPayload` + intents (slice 5.4).
///
/// The payload's sealed-secret fields are **placeholders**; the real intents ride
/// in the parallel [`SecretUpdates`]. Forces `meta.entry_type` to match the
/// variant so a buggy frontend can't smuggle a mismatched pair.
///
/// The placeholders (empty `SecretString` / `None` / empty `Vec`) are ALWAYS
/// overwritten by `resolve_secrets` in `create_entry`/`update_entry` — or the
/// write is rejected — so they never reach `to_encryptable_json`. `Note.content`
/// and the non-`national_id` Identity fields are not sealed and carry real values.
#[allow(clippy::too_many_lines)]
pub fn payload_from_dto(dto: PayloadDto) -> Result<(EntryPayload, SecretUpdates), CommandError> {
    Ok(match dto {
        PayloadDto::Login(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Login;
            let secrets = SecretUpdates::Login {
                password: secret_update_from_dto(d.password),
                recovery_codes: secret_list_update_from_dto(d.recovery_codes),
                totp: totp_update_from_dto(&d.totp),
            };
            let payload = EntryPayload::Login(LoginPayload {
                meta,
                username: d.username,
                password: placeholder_secret(),
                totp_secret: None,
                totp_params: totp_params_from_dto(d.totp_algorithm, d.totp_digits, d.totp_period),
                recovery_codes: Vec::new(),
            });
            (payload, secrets)
        }
        PayloadDto::Card(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Card;
            let secrets = SecretUpdates::Card {
                number: secret_update_from_dto(d.number),
                cvv: secret_update_from_dto(d.cvv),
                pin: secret_update_from_dto(d.pin),
            };
            let payload = EntryPayload::Card(CardPayload {
                meta,
                cardholder_name: d.cardholder_name,
                number: placeholder_secret(),
                expiry_month: d.expiry_month,
                expiry_year: d.expiry_year,
                cvv: placeholder_secret(),
                pin: None,
            });
            (payload, secrets)
        }
        PayloadDto::SshKey(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::SshKey;
            let secrets = SecretUpdates::SshKey {
                private_key_pem: secret_update_from_dto(d.private_key_pem),
                passphrase: secret_update_from_dto(d.passphrase),
            };
            let payload = EntryPayload::SshKey(SshKeyPayload {
                meta,
                private_key_pem: placeholder_secret(),
                passphrase: None,
                public_key: d.public_key,
                fingerprint: d.fingerprint,
                key_type: d.key_type,
            });
            (payload, secrets)
        }
        PayloadDto::ApiKey(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::ApiKey;
            let secrets = SecretUpdates::ApiKey {
                key: secret_update_from_dto(d.key),
                secret: secret_update_from_dto(d.secret),
            };
            let payload = EntryPayload::ApiKey(ApiKeyPayload {
                meta,
                key: placeholder_secret(),
                secret: None,
                endpoint: d.endpoint,
                expiry: d.expiry,
                key_type: d.key_type,
            });
            (payload, secrets)
        }
        PayloadDto::EnvVars(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::EnvVars;
            let secrets = SecretUpdates::EnvVars {
                vars: d.vars.into_iter().map(env_var_update_from_dto).collect(),
            };
            let payload = EntryPayload::EnvVars(EnvVarsPayload {
                meta,
                vars: Vec::new(),
            });
            (payload, secrets)
        }
        PayloadDto::Note(d) => {
            // Note.content is NOT sealed — it is the entry's whole substance.
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Note;
            let payload = EntryPayload::Note(NotePayload {
                meta,
                content: SecretString::from(d.content),
            });
            (payload, SecretUpdates::Note)
        }
        PayloadDto::Document(d) => {
            let blob_nonce = b64_decode_fixed::<24>(&d.blob_nonce_b64)?;
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Document;
            let payload = EntryPayload::Document(DocumentPayload {
                meta,
                filename: d.filename,
                mime_type: d.mime_type,
                size_bytes: d.size_bytes,
                blob_nonce,
            });
            (payload, SecretUpdates::Document)
        }
        PayloadDto::Identity(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Identity;
            let secrets = SecretUpdates::Identity {
                national_id: secret_update_from_dto(d.national_id),
            };
            let payload = EntryPayload::Identity(IdentityPayload {
                meta,
                first_name: d.first_name,
                last_name: d.last_name,
                email: d.email,
                phone: d.phone,
                address: d.address.map(address_from_dto),
                date_of_birth: d.date_of_birth,
                national_id: None,
            });
            (payload, secrets)
        }
        PayloadDto::Folder(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Folder;
            (
                EntryPayload::Folder(FolderPayload { meta }),
                SecretUpdates::Folder,
            )
        }
    })
}

/// Empty placeholder for a sealed secret field, overwritten by `resolve_secrets`
/// before encryption (or the write is rejected). Never persisted.
fn placeholder_secret() -> SecretString {
    SecretString::from(String::new())
}

/// Build a DTO from a domain payload — the **sealed** outbound door (slice 5.4).
///
/// Every sealed-secret field crosses as `SecretUpdateDto::Unchanged` (never the
/// plaintext); optional secrets add a `has_*` presence flag and collections a
/// count/keys. `Note.content` and the non-`national_id` Identity fields are not
/// credential material and keep crossing. Returns `Invalid` for `Unknown`.
pub fn payload_to_dto(p: &EntryPayload) -> Result<PayloadDto, CommandError> {
    Ok(match p {
        EntryPayload::Login(x) => PayloadDto::Login(LoginPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            username: x.username.clone(),
            // Sealed — the password does NOT cross the door.
            password: SecretUpdateDto::Unchanged,
            // The seed does NOT cross either — only its presence + params.
            has_totp: x.totp_secret.is_some(),
            totp_algorithm: totp_algorithm_to_dto(x.totp_params.algorithm),
            totp_digits: x.totp_params.digits,
            totp_period: x.totp_params.period,
            totp: TotpUpdateDto::Unchanged,
            // Sealed — values do not cross; the count does.
            recovery_codes: SecretListUpdateDto::Unchanged,
            recovery_codes_count: u32::try_from(x.recovery_codes.len()).unwrap_or(u32::MAX),
        }),
        EntryPayload::Card(x) => PayloadDto::Card(CardPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            cardholder_name: x.cardholder_name.clone(),
            number: SecretUpdateDto::Unchanged,
            expiry_month: x.expiry_month,
            expiry_year: x.expiry_year,
            cvv: SecretUpdateDto::Unchanged,
            pin: SecretUpdateDto::Unchanged,
            has_pin: x.pin.is_some(),
        }),
        EntryPayload::SshKey(x) => PayloadDto::SshKey(SshKeyPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            private_key_pem: SecretUpdateDto::Unchanged,
            passphrase: SecretUpdateDto::Unchanged,
            has_passphrase: x.passphrase.is_some(),
            public_key: x.public_key.clone(),
            fingerprint: x.fingerprint.clone(),
            key_type: x.key_type.clone(),
        }),
        EntryPayload::ApiKey(x) => PayloadDto::ApiKey(ApiKeyPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            key: SecretUpdateDto::Unchanged,
            secret: SecretUpdateDto::Unchanged,
            has_secret: x.secret.is_some(),
            endpoint: x.endpoint.clone(),
            expiry: x.expiry.clone(),
            key_type: x.key_type.clone(),
        }),
        EntryPayload::EnvVars(x) => PayloadDto::EnvVars(EnvVarsPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            // Keys cross (the schema); each value is sealed as `Unchanged`.
            vars: x
                .vars
                .iter()
                .map(|v| EnvVarUpdateDto {
                    key: v.key.clone(),
                    value: SecretUpdateDto::Unchanged,
                })
                .collect(),
        }),
        EntryPayload::Note(x) => PayloadDto::Note(NotePayloadDto {
            meta: common_meta_to_dto(&x.meta),
            // Not sealed — the note body is the entry's substance.
            content: x.content.expose_secret().to_owned(),
        }),
        EntryPayload::Document(x) => PayloadDto::Document(DocumentPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            filename: x.filename.clone(),
            mime_type: x.mime_type.clone(),
            size_bytes: x.size_bytes,
            blob_nonce_b64: b64_encode(&x.blob_nonce),
        }),
        EntryPayload::Identity(x) => PayloadDto::Identity(IdentityPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            first_name: x.first_name.clone(),
            last_name: x.last_name.clone(),
            email: x.email.clone(),
            phone: x.phone.clone(),
            address: x.address.as_ref().map(address_to_dto),
            date_of_birth: x.date_of_birth.clone(),
            // Sealed — the only credential-material Identity field.
            national_id: SecretUpdateDto::Unchanged,
            has_national_id: x.national_id.is_some(),
        }),
        EntryPayload::Folder(x) => PayloadDto::Folder(FolderPayloadDto {
            meta: common_meta_to_dto(&x.meta),
        }),
        EntryPayload::Unknown(_) => {
            return Err(CommandError::Invalid(
                "cannot convert Unknown entry payload to DTO".into(),
            ));
        }
    })
}

// ---- id helpers --------------------------------------------------------------

#[must_use]
pub fn entry_id_from_str(s: &str) -> EntryId {
    EntryId::from_raw(s.to_owned())
}

#[must_use]
pub fn tag_id_from_str(s: &str) -> TagId {
    TagId::from_raw(s.to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use super::*;
    use crate::dto::common::CommonMetaDto;
    use vedge_core::domain::vault::payloads::{CommonMeta, EnvVar};
    use vedge_core::resolve_secrets;
    use vedge_ipc::EntryTypeDto;

    fn minimal_meta(ty: EntryType) -> CommonMeta {
        CommonMeta::new("demo", ty)
    }

    /// The outbound DTO, serialized — the exact bytes that cross to WASM.
    fn json_of(p: &EntryPayload) -> String {
        serde_json::to_string(&payload_to_dto(p).unwrap()).unwrap()
    }

    fn meta_dto(ty: EntryTypeDto) -> CommonMetaDto {
        CommonMetaDto {
            name: "demo".into(),
            entry_type: ty,
            url: None,
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            notes: None,
            color: None,
            icon: None,
            sort_order: 0,
        }
    }

    // ---- The per-field door (slice 5.4): no secret crosses on payload_to_dto ----
    // The acceptance bar: serialize each payload DTO and assert every secret
    // string appears NOWHERE in the JSON. Sentinels are distinctive so a stray
    // key would fail the `contains` check.

    #[test]
    fn login_seals_password_recovery_and_totp_seed() {
        let p = EntryPayload::Login(LoginPayload {
            meta: minimal_meta(EntryType::Login),
            username: "alice".into(),
            password: SecretString::from("SEKRIT-pw"),
            totp_secret: Some(SecretString::from("SEKRITSEED")),
            totp_params: vedge_core::TotpParams::default(),
            recovery_codes: vec![
                SecretString::from("SEKRIT-rc-1"),
                SecretString::from("SEKRIT-rc-2"),
            ],
        });
        let json = json_of(&p);
        assert!(!json.contains("SEKRIT-pw"), "password must not cross");
        assert!(!json.contains("SEKRITSEED"), "totp seed must not cross");
        assert!(!json.contains("SEKRIT-rc"), "recovery codes must not cross");
        assert!(json.contains("alice"), "username (non-secret) crosses");

        let PayloadDto::Login(d) = payload_to_dto(&p).unwrap() else {
            panic!("wrong variant")
        };
        assert!(d.has_totp);
        assert_eq!(d.recovery_codes_count, 2, "a count crosses, not the codes");
        assert!(matches!(d.password, SecretUpdateDto::Unchanged));
        assert!(matches!(d.recovery_codes, SecretListUpdateDto::Unchanged));
    }

    #[test]
    fn card_seals_number_cvv_pin() {
        let p = EntryPayload::Card(CardPayload {
            meta: minimal_meta(EntryType::Card),
            cardholder_name: "Alice A".into(),
            number: SecretString::from("SEKRIT-4111"),
            expiry_month: 12,
            expiry_year: 2030,
            cvv: SecretString::from("SEKRIT-cvv"),
            pin: Some(SecretString::from("SEKRIT-pin")),
        });
        let json = json_of(&p);
        for s in ["SEKRIT-4111", "SEKRIT-cvv", "SEKRIT-pin"] {
            assert!(!json.contains(s), "{s} must not cross");
        }
        assert!(json.contains("Alice A"), "cardholder name crosses");
        let PayloadDto::Card(d) = payload_to_dto(&p).unwrap() else {
            panic!("wrong variant")
        };
        assert!(d.has_pin);
    }

    #[test]
    fn ssh_seals_private_key_and_passphrase() {
        let p = EntryPayload::SshKey(SshKeyPayload {
            meta: minimal_meta(EntryType::SshKey),
            private_key_pem: SecretString::from("SEKRIT-BEGIN-KEY"),
            passphrase: Some(SecretString::from("SEKRIT-pp")),
            public_key: "ssh-ed25519 AAAA".into(),
            fingerprint: "SHA256:abc".into(),
            key_type: "ed25519".into(),
        });
        let json = json_of(&p);
        assert!(
            !json.contains("SEKRIT-BEGIN-KEY"),
            "private key must not cross"
        );
        assert!(!json.contains("SEKRIT-pp"), "passphrase must not cross");
        assert!(json.contains("ssh-ed25519 AAAA"), "public key crosses");
        let PayloadDto::SshKey(d) = payload_to_dto(&p).unwrap() else {
            panic!("wrong variant")
        };
        assert!(d.has_passphrase);
    }

    #[test]
    fn api_key_seals_key_and_secret() {
        let p = EntryPayload::ApiKey(ApiKeyPayload {
            meta: minimal_meta(EntryType::ApiKey),
            key: SecretString::from("SEKRIT-sk"),
            secret: Some(SecretString::from("SEKRIT-shh")),
            endpoint: Some("https://api".into()),
            expiry: None,
            key_type: Some("bearer".into()),
        });
        let json = json_of(&p);
        assert!(!json.contains("SEKRIT-sk"), "key must not cross");
        assert!(!json.contains("SEKRIT-shh"), "secret must not cross");
        assert!(json.contains("https://api"), "endpoint crosses");
        let PayloadDto::ApiKey(d) = payload_to_dto(&p).unwrap() else {
            panic!("wrong variant")
        };
        assert!(d.has_secret);
    }

    #[test]
    fn env_vars_seal_values_but_keep_keys() {
        let p = EntryPayload::EnvVars(EnvVarsPayload {
            meta: minimal_meta(EntryType::EnvVars),
            vars: vec![
                EnvVar {
                    key: "DB_URL".into(),
                    value: SecretString::from("SEKRIT-db"),
                },
                EnvVar {
                    key: "TOKEN".into(),
                    value: SecretString::from("SEKRIT-tok"),
                },
            ],
        });
        let json = json_of(&p);
        assert!(
            json.contains("DB_URL") && json.contains("TOKEN"),
            "keys are the schema — they cross"
        );
        assert!(
            !json.contains("SEKRIT-db") && !json.contains("SEKRIT-tok"),
            "values do not cross"
        );
    }

    #[test]
    fn note_content_and_identity_pii_still_cross_only_national_id_sealed() {
        // Note.content is the entry's whole substance — NOT sealed (①'s guard).
        let note = EntryPayload::Note(NotePayload {
            meta: minimal_meta(EntryType::Note),
            content: SecretString::from("MY-NOTE-BODY"),
        });
        assert!(json_of(&note).contains("MY-NOTE-BODY"), "note body crosses");

        // Identity PII crosses; only national_id is credential material.
        let ident = EntryPayload::Identity(IdentityPayload {
            meta: minimal_meta(EntryType::Identity),
            first_name: "Alice".into(),
            last_name: "Anderson".into(),
            email: "alice@example.com".into(),
            phone: Some("+1".into()),
            address: None,
            date_of_birth: Some("1990-01-01".into()),
            national_id: Some(SecretString::from("SEKRIT-nid")),
        });
        let json = json_of(&ident);
        assert!(
            json.contains("Alice") && json.contains("alice@example.com"),
            "identity PII crosses (not credential material)"
        );
        assert!(!json.contains("SEKRIT-nid"), "national_id is sealed");
        let PayloadDto::Identity(d) = payload_to_dto(&ident).unwrap() else {
            panic!("wrong variant")
        };
        assert!(d.has_national_id);
    }

    // ---- Inbound: intents resolve into values ----

    #[test]
    fn inbound_set_materializes_the_value_on_create() {
        // A create DTO carries `Set`/`Clear` intents; `payload_from_dto` gives
        // placeholders + intents; `resolve_secrets(None)` fills them.
        let dto = PayloadDto::Card(CardPayloadDto {
            meta: meta_dto(EntryTypeDto::Card),
            cardholder_name: "A".into(),
            number: SecretUpdateDto::Set("4111".into()),
            expiry_month: 1,
            expiry_year: 2030,
            cvv: SecretUpdateDto::Set("123".into()),
            pin: SecretUpdateDto::Clear,
            has_pin: false,
        });
        let (mut payload, secrets) = payload_from_dto(dto).unwrap();
        resolve_secrets(&mut payload, secrets, None).unwrap();
        let EntryPayload::Card(c) = payload else {
            panic!("wrong variant")
        };
        assert_eq!(c.number.expose_secret(), "4111");
        assert_eq!(c.cvv.expose_secret(), "123");
        assert!(c.pin.is_none(), "Clear on an optional → None");
    }

    #[test]
    fn inbound_unchanged_carries_the_original_across_the_seal() {
        // The full loop: a real payload → sealed DTO (all `Unchanged`) →
        // placeholders → resolve against the ORIGINAL restores every secret.
        let orig = EntryPayload::Card(CardPayload {
            meta: minimal_meta(EntryType::Card),
            cardholder_name: "A".into(),
            number: SecretString::from("4111"),
            expiry_month: 1,
            expiry_year: 2030,
            cvv: SecretString::from("123"),
            pin: Some(SecretString::from("9999")),
        });
        let (mut payload, secrets) = payload_from_dto(payload_to_dto(&orig).unwrap()).unwrap();
        resolve_secrets(&mut payload, secrets, Some(&orig)).unwrap();
        let EntryPayload::Card(c) = payload else {
            panic!("wrong variant")
        };
        assert_eq!(c.number.expose_secret(), "4111");
        assert_eq!(c.cvv.expose_secret(), "123");
        assert_eq!(c.pin.unwrap().expose_secret(), "9999");
    }

    #[test]
    fn document_round_trips_nonce_as_b64() {
        let nonce = [7u8; 24];
        let p = EntryPayload::Document(DocumentPayload {
            meta: minimal_meta(EntryType::Document),
            filename: "a.pdf".into(),
            mime_type: "application/pdf".into(),
            size_bytes: 12_345,
            blob_nonce: nonce,
        });
        let (back, _) = payload_from_dto(payload_to_dto(&p).unwrap()).unwrap();
        let EntryPayload::Document(b) = back else {
            panic!("wrong variant")
        };
        assert_eq!(b.blob_nonce, nonce);
        assert_eq!(b.filename, "a.pdf");
        assert_eq!(b.size_bytes, 12_345);
    }

    #[test]
    fn folder_round_trips_through_dto() {
        let p = EntryPayload::Folder(FolderPayload {
            meta: minimal_meta(EntryType::Folder),
        });
        let (back, _) = payload_from_dto(payload_to_dto(&p).unwrap()).unwrap();
        assert!(matches!(back, EntryPayload::Folder(_)));
        assert_eq!(back.meta().entry_type, EntryType::Folder);
    }

    #[test]
    fn payload_dto_forces_entry_type_match() {
        let mut wrong_meta = meta_dto(EntryTypeDto::Login); // wrong
        wrong_meta.name = "x".into();
        let dto = PayloadDto::Note(NotePayloadDto {
            meta: wrong_meta,
            content: "body".into(),
        });
        let (back, _) = payload_from_dto(dto).unwrap();
        assert_eq!(back.meta().entry_type, EntryType::Note);
    }

    #[test]
    fn unknown_payload_cannot_serialize() {
        use vedge_core::domain::vault::payloads::UnknownPayload;
        let p = EntryPayload::Unknown(UnknownPayload {
            meta: minimal_meta(EntryType::Unknown("Passkey".into())),
            unknown_fields: serde_json::json!({"foo":"bar"}),
        });
        assert!(payload_to_dto(&p).is_err());
    }

    #[test]
    fn helper_id_constructors_preserve_input() {
        assert_eq!(entry_id_from_str("abc").as_str(), "abc");
        assert_eq!(tag_id_from_str("xyz").as_str(), "xyz");
    }
}
