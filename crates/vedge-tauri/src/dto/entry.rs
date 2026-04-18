//! Entry-related DTOs: the read-side `IndexEntryDto`, the write-side
//! `PayloadDto` family, and the converters that shuttle them in and out of
//! `vedge-core` domain types.
//!
//! ## Secret handling
//!
//! Secrets cross as plain `String` / `Vec<String>`. The `into_domain`
//! converters wrap each one in `SecretString` on its way into the domain
//! layer; the DTO value drops immediately after, along with the raw bytes.
//!
//! The reverse direction (`from_domain`) exists only for `export_entry`
//! flows that need to surface plaintext to the JS side — it materializes
//! secret strings via `ExposeSecret` one last time, which is acceptable
//! because the JS layer is the intended consumer.

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::index::IndexEntry;
use vedge_core::domain::vault::payloads::{
    Address, ApiKeyPayload, CardPayload, DocumentPayload, EntryPayload, EntryType, EnvVar,
    EnvVarsPayload, FolderPayload, IdentityPayload, LoginPayload, NotePayload, SshKeyPayload,
};

use crate::dto::common::{b64_decode_fixed, b64_encode, ts_to_string, CommonMetaDto, EntryTypeDto};
use crate::error::CommandError;

// ---- IndexEntryDto -----------------------------------------------------------

/// Read-side projection of an entry, safe to hand to the UI. Non-secret
/// fields only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntryDto {
    pub id: String,
    pub name: String,
    pub entry_type: EntryTypeDto,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub favicon_url: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    pub is_favorite: bool,
    pub is_trashed: bool,
    pub cipher_suite: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub accessed_at: Option<String>,
}

impl From<&IndexEntry> for IndexEntryDto {
    fn from(e: &IndexEntry) -> Self {
        Self {
            id: e.id.as_str().to_owned(),
            name: e.name.clone(),
            entry_type: (&e.entry_type).into(),
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
}

// ---- AddressDto --------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressDto {
    pub line1: String,
    #[serde(default)]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default)]
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
}

impl From<&Address> for AddressDto {
    fn from(a: &Address) -> Self {
        Self {
            line1: a.line1.clone(),
            line2: a.line2.clone(),
            city: a.city.clone(),
            state: a.state.clone(),
            postal_code: a.postal_code.clone(),
            country: a.country.clone(),
        }
    }
}

impl From<AddressDto> for Address {
    fn from(a: AddressDto) -> Self {
        Self {
            line1: a.line1,
            line2: a.line2,
            city: a.city,
            state: a.state,
            postal_code: a.postal_code,
            country: a.country,
        }
    }
}

// ---- EnvVarDto ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarDto {
    pub key: String,
    pub value: String,
}

impl EnvVarDto {
    fn into_domain(self) -> EnvVar {
        EnvVar {
            key: self.key,
            value: SecretString::from(self.value),
        }
    }

    fn from_domain(v: &EnvVar) -> Self {
        Self {
            key: v.key.clone(),
            value: v.value.expose_secret().to_owned(),
        }
    }
}

// ---- Per-variant payload DTOs -----------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPayloadDto {
    pub meta: CommonMetaDto,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub totp_secret: Option<String>,
    #[serde(default)]
    pub recovery_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardPayloadDto {
    pub meta: CommonMetaDto,
    pub cardholder_name: String,
    pub number: String,
    pub expiry_month: u8,
    pub expiry_year: u16,
    pub cvv: String,
    #[serde(default)]
    pub pin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyPayloadDto {
    pub meta: CommonMetaDto,
    pub private_key_pem: String,
    #[serde(default)]
    pub passphrase: Option<String>,
    pub public_key: String,
    pub fingerprint: String,
    pub key_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyPayloadDto {
    pub meta: CommonMetaDto,
    pub key: String,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub expiry: Option<String>,
    #[serde(default)]
    pub key_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarsPayloadDto {
    pub meta: CommonMetaDto,
    #[serde(default)]
    pub vars: Vec<EnvVarDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotePayloadDto {
    pub meta: CommonMetaDto,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPayloadDto {
    pub meta: CommonMetaDto,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    /// Base64 of the 24-byte nonce used to encrypt the sidecar blob.
    pub blob_nonce_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPayloadDto {
    pub meta: CommonMetaDto,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub address: Option<AddressDto>,
    #[serde(default)]
    pub date_of_birth: Option<String>,
    #[serde(default)]
    pub national_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPayloadDto {
    pub meta: CommonMetaDto,
}

// ---- PayloadDto (adjacently tagged union) -----------------------------------

/// Tagged union over every writable payload variant. JS shape:
/// `{ "entry_type": "Login", "data": { ... } }`.
///
/// `Unknown` has no DTO — it's a read-side-only domain variant that
/// represents an entry whose schema we don't recognise. `create_entry` /
/// `update_entry` can't produce one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "entry_type", content = "data")]
pub enum PayloadDto {
    Login(LoginPayloadDto),
    Card(CardPayloadDto),
    SshKey(SshKeyPayloadDto),
    ApiKey(ApiKeyPayloadDto),
    EnvVars(EnvVarsPayloadDto),
    Note(NotePayloadDto),
    Document(DocumentPayloadDto),
    Identity(IdentityPayloadDto),
    Folder(FolderPayloadDto),
}

impl PayloadDto {
    /// Convert to a domain `EntryPayload`, forcing `meta.entry_type` to
    /// match the variant so a buggy / tampering frontend can't smuggle a
    /// mismatched pair.
    #[allow(clippy::too_many_lines)] // one arm per variant — each arm is short
    pub fn into_domain(self) -> Result<EntryPayload, CommandError> {
        match self {
            Self::Login(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Login;
                Ok(EntryPayload::Login(LoginPayload {
                    meta,
                    username: dto.username,
                    password: SecretString::from(dto.password),
                    totp_secret: dto.totp_secret.map(SecretString::from),
                    recovery_codes: dto
                        .recovery_codes
                        .into_iter()
                        .map(SecretString::from)
                        .collect(),
                }))
            }
            Self::Card(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Card;
                Ok(EntryPayload::Card(CardPayload {
                    meta,
                    cardholder_name: dto.cardholder_name,
                    number: SecretString::from(dto.number),
                    expiry_month: dto.expiry_month,
                    expiry_year: dto.expiry_year,
                    cvv: SecretString::from(dto.cvv),
                    pin: dto.pin.map(SecretString::from),
                }))
            }
            Self::SshKey(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::SshKey;
                Ok(EntryPayload::SshKey(SshKeyPayload {
                    meta,
                    private_key_pem: SecretString::from(dto.private_key_pem),
                    passphrase: dto.passphrase.map(SecretString::from),
                    public_key: dto.public_key,
                    fingerprint: dto.fingerprint,
                    key_type: dto.key_type,
                }))
            }
            Self::ApiKey(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::ApiKey;
                Ok(EntryPayload::ApiKey(ApiKeyPayload {
                    meta,
                    key: SecretString::from(dto.key),
                    secret: dto.secret.map(SecretString::from),
                    endpoint: dto.endpoint,
                    expiry: dto.expiry,
                    key_type: dto.key_type,
                }))
            }
            Self::EnvVars(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::EnvVars;
                Ok(EntryPayload::EnvVars(EnvVarsPayload {
                    meta,
                    vars: dto.vars.into_iter().map(EnvVarDto::into_domain).collect(),
                }))
            }
            Self::Note(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Note;
                Ok(EntryPayload::Note(NotePayload {
                    meta,
                    content: SecretString::from(dto.content),
                }))
            }
            Self::Document(dto) => {
                let blob_nonce = b64_decode_fixed::<24>(&dto.blob_nonce_b64)?;
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Document;
                Ok(EntryPayload::Document(DocumentPayload {
                    meta,
                    filename: dto.filename,
                    mime_type: dto.mime_type,
                    size_bytes: dto.size_bytes,
                    blob_nonce,
                }))
            }
            Self::Identity(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Identity;
                Ok(EntryPayload::Identity(IdentityPayload {
                    meta,
                    first_name: dto.first_name,
                    last_name: dto.last_name,
                    email: dto.email,
                    phone: dto.phone,
                    address: dto.address.map(Into::into),
                    date_of_birth: dto.date_of_birth,
                    national_id: dto.national_id.map(SecretString::from),
                }))
            }
            Self::Folder(dto) => {
                let mut meta = dto.meta.into_domain();
                meta.entry_type = EntryType::Folder;
                Ok(EntryPayload::Folder(FolderPayload { meta }))
            }
        }
    }

    /// Build a DTO from a domain payload. Returns `Invalid` for `Unknown`
    /// variants — the shell has no stable wire-shape for unrecognised
    /// entry types; callers should surface the underlying entry rather
    /// than trying to convert it.
    pub fn from_domain(p: &EntryPayload) -> Result<Self, CommandError> {
        Ok(match p {
            EntryPayload::Login(x) => Self::Login(LoginPayloadDto {
                meta: (&x.meta).into(),
                username: x.username.clone(),
                password: x.password.expose_secret().to_owned(),
                totp_secret: x.totp_secret.as_ref().map(|s| s.expose_secret().to_owned()),
                recovery_codes: x
                    .recovery_codes
                    .iter()
                    .map(|s| s.expose_secret().to_owned())
                    .collect(),
            }),
            EntryPayload::Card(x) => Self::Card(CardPayloadDto {
                meta: (&x.meta).into(),
                cardholder_name: x.cardholder_name.clone(),
                number: x.number.expose_secret().to_owned(),
                expiry_month: x.expiry_month,
                expiry_year: x.expiry_year,
                cvv: x.cvv.expose_secret().to_owned(),
                pin: x.pin.as_ref().map(|s| s.expose_secret().to_owned()),
            }),
            EntryPayload::SshKey(x) => Self::SshKey(SshKeyPayloadDto {
                meta: (&x.meta).into(),
                private_key_pem: x.private_key_pem.expose_secret().to_owned(),
                passphrase: x.passphrase.as_ref().map(|s| s.expose_secret().to_owned()),
                public_key: x.public_key.clone(),
                fingerprint: x.fingerprint.clone(),
                key_type: x.key_type.clone(),
            }),
            EntryPayload::ApiKey(x) => Self::ApiKey(ApiKeyPayloadDto {
                meta: (&x.meta).into(),
                key: x.key.expose_secret().to_owned(),
                secret: x.secret.as_ref().map(|s| s.expose_secret().to_owned()),
                endpoint: x.endpoint.clone(),
                expiry: x.expiry.clone(),
                key_type: x.key_type.clone(),
            }),
            EntryPayload::EnvVars(x) => Self::EnvVars(EnvVarsPayloadDto {
                meta: (&x.meta).into(),
                vars: x.vars.iter().map(EnvVarDto::from_domain).collect(),
            }),
            EntryPayload::Note(x) => Self::Note(NotePayloadDto {
                meta: (&x.meta).into(),
                content: x.content.expose_secret().to_owned(),
            }),
            EntryPayload::Document(x) => Self::Document(DocumentPayloadDto {
                meta: (&x.meta).into(),
                filename: x.filename.clone(),
                mime_type: x.mime_type.clone(),
                size_bytes: x.size_bytes,
                blob_nonce_b64: b64_encode(&x.blob_nonce),
            }),
            EntryPayload::Identity(x) => Self::Identity(IdentityPayloadDto {
                meta: (&x.meta).into(),
                first_name: x.first_name.clone(),
                last_name: x.last_name.clone(),
                email: x.email.clone(),
                phone: x.phone.clone(),
                address: x.address.as_ref().map(AddressDto::from),
                date_of_birth: x.date_of_birth.clone(),
                national_id: x.national_id.as_ref().map(|s| s.expose_secret().to_owned()),
            }),
            EntryPayload::Folder(x) => Self::Folder(FolderPayloadDto {
                meta: (&x.meta).into(),
            }),
            EntryPayload::Unknown(_) => {
                return Err(CommandError::Invalid(
                    "cannot convert Unknown entry payload to DTO".into(),
                ));
            }
        })
    }
}

// ---- id helpers --------------------------------------------------------------

/// Wrap a string as an `EntryId`. The shell never validates ULID format —
/// `VaultRepository` does that on lookup.
#[must_use]
pub fn entry_id_from_str(s: &str) -> EntryId {
    EntryId::from_raw(s.to_owned())
}

/// Wrap a string as a `TagId`. Same as `entry_id_from_str` for tags.
#[must_use]
pub fn tag_id_from_str(s: &str) -> TagId {
    TagId::from_raw(s.to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use super::*;
    use vedge_core::domain::vault::payloads::CommonMeta;

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
        let dto = PayloadDto::from_domain(&p).unwrap();
        let back = dto.into_domain().unwrap();
        let EntryPayload::Login(b) = back else { panic!("wrong variant") };
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
        let dto = PayloadDto::from_domain(&p).unwrap();
        let back = dto.into_domain().unwrap();
        let EntryPayload::Document(b) = back else { panic!("wrong variant") };
        assert_eq!(b.blob_nonce, nonce);
        assert_eq!(b.filename, "a.pdf");
        assert_eq!(b.size_bytes, 12_345);
    }

    #[test]
    fn payload_dto_forces_entry_type_match() {
        // Frontend sends Note DTO but meta claims Login — shell rewrites
        // meta.entry_type to Note on conversion.
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
        let back = dto.into_domain().unwrap();
        assert_eq!(back.meta().entry_type, EntryType::Note);
    }

    #[test]
    fn unknown_payload_cannot_serialize() {
        use vedge_core::domain::vault::payloads::UnknownPayload;
        let p = EntryPayload::Unknown(UnknownPayload {
            meta: minimal_meta(EntryType::Unknown("Passkey".into())),
            unknown_fields: serde_json::json!({"foo":"bar"}),
        });
        assert!(PayloadDto::from_domain(&p).is_err());
    }

    #[test]
    fn helper_id_constructors_preserve_input() {
        assert_eq!(entry_id_from_str("abc").as_str(), "abc");
        assert_eq!(tag_id_from_str("xyz").as_str(), "xyz");
    }
}
