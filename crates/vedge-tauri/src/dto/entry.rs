//! Entry DTO conversion layer (wire types from `vedge_ipc`).

pub use vedge_ipc::{
    AddressDto, ApiKeyPayloadDto, CardPayloadDto, DocumentPayloadDto, EnvVarDto, EnvVarsPayloadDto,
    FolderPayloadDto, IdentityPayloadDto, IndexEntryDto, LoginPayloadDto, NotePayloadDto,
    PayloadDto, SshKeyPayloadDto,
};

use secrecy::{ExposeSecret, SecretString};

use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::index::IndexEntry;
use vedge_core::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, DocumentPayload, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
};

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
        is_trashed: e.is_trashed,
        cipher_suite: e.cipher_suite,
        created_at: ts_to_string(e.created_at),
        updated_at: ts_to_string(e.updated_at),
        accessed_at: e.accessed_at.map(ts_to_string),
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

// ---- EnvVar ------------------------------------------------------------------

fn env_var_from_dto(v: EnvVarDto) -> EnvVar {
    EnvVar {
        key: v.key,
        value: SecretString::from(v.value),
    }
}

fn env_var_to_dto(v: &EnvVar) -> EnvVarDto {
    EnvVarDto {
        key: v.key.clone(),
        value: v.value.expose_secret().to_owned(),
    }
}

// ---- Payload round-trip ------------------------------------------------------

/// Convert a write-side DTO into a domain `EntryPayload`, forcing
/// `meta.entry_type` to match the variant so a buggy frontend can't smuggle
/// a mismatched pair.
#[allow(clippy::too_many_lines)]
pub fn payload_from_dto(dto: PayloadDto) -> Result<EntryPayload, CommandError> {
    Ok(match dto {
        PayloadDto::Login(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Login;
            EntryPayload::Login(LoginPayload {
                meta,
                username: d.username,
                password: SecretString::from(d.password),
                totp_secret: d.totp_secret.map(SecretString::from),
                recovery_codes: d.recovery_codes.into_iter().map(SecretString::from).collect(),
            })
        }
        PayloadDto::Card(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Card;
            EntryPayload::Card(CardPayload {
                meta,
                cardholder_name: d.cardholder_name,
                number: SecretString::from(d.number),
                expiry_month: d.expiry_month,
                expiry_year: d.expiry_year,
                cvv: SecretString::from(d.cvv),
                pin: d.pin.map(SecretString::from),
            })
        }
        PayloadDto::SshKey(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::SshKey;
            EntryPayload::SshKey(SshKeyPayload {
                meta,
                private_key_pem: SecretString::from(d.private_key_pem),
                passphrase: d.passphrase.map(SecretString::from),
                public_key: d.public_key,
                fingerprint: d.fingerprint,
                key_type: d.key_type,
            })
        }
        PayloadDto::ApiKey(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::ApiKey;
            EntryPayload::ApiKey(ApiKeyPayload {
                meta,
                key: SecretString::from(d.key),
                secret: d.secret.map(SecretString::from),
                endpoint: d.endpoint,
                expiry: d.expiry,
                key_type: d.key_type,
            })
        }
        PayloadDto::EnvVars(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::EnvVars;
            EntryPayload::EnvVars(EnvVarsPayload {
                meta,
                vars: d.vars.into_iter().map(env_var_from_dto).collect(),
            })
        }
        PayloadDto::Note(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Note;
            EntryPayload::Note(NotePayload {
                meta,
                content: SecretString::from(d.content),
            })
        }
        PayloadDto::Document(d) => {
            let blob_nonce = b64_decode_fixed::<24>(&d.blob_nonce_b64)?;
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Document;
            EntryPayload::Document(DocumentPayload {
                meta,
                filename: d.filename,
                mime_type: d.mime_type,
                size_bytes: d.size_bytes,
                blob_nonce,
            })
        }
        PayloadDto::Identity(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Identity;
            EntryPayload::Identity(IdentityPayload {
                meta,
                first_name: d.first_name,
                last_name: d.last_name,
                email: d.email,
                phone: d.phone,
                address: d.address.map(address_from_dto),
                date_of_birth: d.date_of_birth,
                national_id: d.national_id.map(SecretString::from),
            })
        }
        PayloadDto::Folder(d) => {
            let mut meta = common_meta_from_dto(d.meta);
            meta.entry_type = EntryType::Folder;
            EntryPayload::Folder(FolderPayload { meta })
        }
    })
}

/// Build a DTO from a domain payload. Returns `Invalid` for `Unknown`
/// variants — no stable wire-shape for unrecognized entry types.
pub fn payload_to_dto(p: &EntryPayload) -> Result<PayloadDto, CommandError> {
    Ok(match p {
        EntryPayload::Login(x) => PayloadDto::Login(LoginPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            username: x.username.clone(),
            password: x.password.expose_secret().to_owned(),
            totp_secret: x.totp_secret.as_ref().map(|s| s.expose_secret().to_owned()),
            recovery_codes: x
                .recovery_codes
                .iter()
                .map(|s| s.expose_secret().to_owned())
                .collect(),
        }),
        EntryPayload::Card(x) => PayloadDto::Card(CardPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            cardholder_name: x.cardholder_name.clone(),
            number: x.number.expose_secret().to_owned(),
            expiry_month: x.expiry_month,
            expiry_year: x.expiry_year,
            cvv: x.cvv.expose_secret().to_owned(),
            pin: x.pin.as_ref().map(|s| s.expose_secret().to_owned()),
        }),
        EntryPayload::SshKey(x) => PayloadDto::SshKey(SshKeyPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            private_key_pem: x.private_key_pem.expose_secret().to_owned(),
            passphrase: x.passphrase.as_ref().map(|s| s.expose_secret().to_owned()),
            public_key: x.public_key.clone(),
            fingerprint: x.fingerprint.clone(),
            key_type: x.key_type.clone(),
        }),
        EntryPayload::ApiKey(x) => PayloadDto::ApiKey(ApiKeyPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            key: x.key.expose_secret().to_owned(),
            secret: x.secret.as_ref().map(|s| s.expose_secret().to_owned()),
            endpoint: x.endpoint.clone(),
            expiry: x.expiry.clone(),
            key_type: x.key_type.clone(),
        }),
        EntryPayload::EnvVars(x) => PayloadDto::EnvVars(EnvVarsPayloadDto {
            meta: common_meta_to_dto(&x.meta),
            vars: x.vars.iter().map(env_var_to_dto).collect(),
        }),
        EntryPayload::Note(x) => PayloadDto::Note(NotePayloadDto {
            meta: common_meta_to_dto(&x.meta),
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
            national_id: x.national_id.as_ref().map(|s| s.expose_secret().to_owned()),
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
    use vedge_core::domain::vault::payloads::CommonMeta;
    use vedge_ipc::EntryTypeDto;

    fn minimal_meta(ty: EntryType) -> CommonMeta {
        CommonMeta::new("demo", ty)
    }

    #[test]
    fn login_round_trips_through_dto() {
        let p = EntryPayload::Login(LoginPayload {
            meta: minimal_meta(EntryType::Login),
            username: "alice".into(),
            password: SecretString::from("hunter2"),
            totp_secret: Some(SecretString::from("JBSWY3DPEHPK3PXP")),
            recovery_codes: vec![SecretString::from("code-1")],
        });
        let dto = payload_to_dto(&p).unwrap();
        let back = payload_from_dto(dto).unwrap();
        let EntryPayload::Login(b) = back else {
            panic!("wrong variant")
        };
        assert_eq!(b.username, "alice");
        assert_eq!(b.password.expose_secret(), "hunter2");
        assert_eq!(b.totp_secret.unwrap().expose_secret(), "JBSWY3DPEHPK3PXP");
        assert_eq!(b.recovery_codes[0].expose_secret(), "code-1");
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
        let dto = payload_to_dto(&p).unwrap();
        let back = payload_from_dto(dto).unwrap();
        let EntryPayload::Document(b) = back else {
            panic!("wrong variant")
        };
        assert_eq!(b.blob_nonce, nonce);
        assert_eq!(b.filename, "a.pdf");
        assert_eq!(b.size_bytes, 12_345);
    }

    #[test]
    fn payload_dto_forces_entry_type_match() {
        let wrong_meta = CommonMetaDto {
            name: "x".into(),
            entry_type: EntryTypeDto::Login, // wrong
            url: None,
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            notes: None,
        };
        let dto = PayloadDto::Note(NotePayloadDto {
            meta: wrong_meta,
            content: "body".into(),
        });
        let back = payload_from_dto(dto).unwrap();
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
