use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroizing;

use crate::domain::vault::errors::VaultError;
use crate::domain::vault::payloads::api_key::ApiKeyPayload;
use crate::domain::vault::payloads::card::CardPayload;
use crate::domain::vault::payloads::common_meta::{CURRENT_PAYLOAD_SCHEMA, CommonMeta};
use crate::domain::vault::payloads::document::DocumentPayload;
use crate::domain::vault::payloads::entry_type::EntryType;
use crate::domain::vault::payloads::env_vars::EnvVarsPayload;
use crate::domain::vault::payloads::folder::FolderPayload;
use crate::domain::vault::payloads::identity::IdentityPayload;
use crate::domain::vault::payloads::login::LoginPayload;
use crate::domain::vault::payloads::note::NotePayload;
use crate::domain::vault::payloads::ssh_key::SshKeyPayload;
use crate::domain::vault::payloads::unknown::UnknownPayload;

#[derive(Debug, Clone)]
pub enum EntryPayload {
    Login(LoginPayload),
    Card(CardPayload),
    SshKey(SshKeyPayload),
    ApiKey(ApiKeyPayload),
    EnvVars(EnvVarsPayload),
    Note(NotePayload),
    Document(DocumentPayload),
    Identity(IdentityPayload),
    Folder(FolderPayload),
    Unknown(UnknownPayload),
}

impl EntryPayload {
    #[must_use]
    pub const fn meta(&self) -> &CommonMeta {
        match self {
            Self::Login(p) => &p.meta,
            Self::Card(p) => &p.meta,
            Self::SshKey(p) => &p.meta,
            Self::ApiKey(p) => &p.meta,
            Self::EnvVars(p) => &p.meta,
            Self::Note(p) => &p.meta,
            Self::Document(p) => &p.meta,
            Self::Identity(p) => &p.meta,
            Self::Folder(p) => &p.meta,
            Self::Unknown(p) => &p.meta,
        }
    }

    pub const fn meta_mut(&mut self) -> &mut CommonMeta {
        match self {
            Self::Login(p) => &mut p.meta,
            Self::Card(p) => &mut p.meta,
            Self::SshKey(p) => &mut p.meta,
            Self::ApiKey(p) => &mut p.meta,
            Self::EnvVars(p) => &mut p.meta,
            Self::Note(p) => &mut p.meta,
            Self::Document(p) => &mut p.meta,
            Self::Identity(p) => &mut p.meta,
            Self::Folder(p) => &mut p.meta,
            Self::Unknown(p) => &mut p.meta,
        }
    }

    #[must_use]
    pub const fn entry_type(&self) -> &EntryType {
        &self.meta().entry_type
    }

    /// Serialize to JSON bytes for encryption. Intent-explicit name — grep for
    /// this to find every encryption boundary. Returns `Zeroizing<Vec<u8>>`
    /// because the bytes contain serialized secrets.
    ///
    /// Refuses to serialize `Unknown` — the schema is not ours to write back.
    pub fn to_encryptable_json(&self) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        if let Self::Unknown(u) = self {
            return Err(VaultError::UnsupportedEntryType(format!(
                "cannot re-serialize unknown entry type: {:?}",
                u.meta.entry_type
            )));
        }
        serde_json::to_vec(self)
            .map(Zeroizing::new)
            .map_err(|e| VaultError::MalformedPayload(e.to_string()))
    }

    /// Deserialize from decrypted plaintext bytes. Enforces `payload_schema`
    /// and routes unknown entry types into `Unknown(UnknownPayload)`.
    pub fn from_decrypted_json(bytes: &[u8]) -> Result<Self, VaultError> {
        serde_json::from_slice(bytes).map_err(|e| {
            // Preserve the typed errors we raise from Deserialize; otherwise
            // report as malformed.
            let msg = e.to_string();
            extract_unsupported_schema(&msg).map_or(
                VaultError::MalformedPayload(msg),
                VaultError::UnsupportedPayloadSchema,
            )
        })
    }
}

fn extract_unsupported_schema(msg: &str) -> Option<u32> {
    // Deserialize error messages look like "unsupported payload_schema: 2".
    let rest = msg.split_once("unsupported payload_schema: ")?.1;
    let token = rest.split_whitespace().next()?;
    token
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
}

impl Serialize for EntryPayload {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Login(p) => p.serialize(s),
            Self::Card(p) => p.serialize(s),
            Self::SshKey(p) => p.serialize(s),
            Self::ApiKey(p) => p.serialize(s),
            Self::EnvVars(p) => p.serialize(s),
            Self::Note(p) => p.serialize(s),
            Self::Document(p) => p.serialize(s),
            Self::Identity(p) => p.serialize(s),
            Self::Folder(p) => p.serialize(s),
            Self::Unknown(_) => Err(serde::ser::Error::custom(
                "EntryPayload::Unknown cannot be serialized — unknown schema",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for EntryPayload {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(d)?;

        let schema_u64 = value
            .get("payload_schema")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1);
        let schema = u32::try_from(schema_u64).unwrap_or(u32::MAX);
        if schema > CURRENT_PAYLOAD_SCHEMA {
            return Err(D::Error::custom(format!(
                "unsupported payload_schema: {schema}"
            )));
        }

        let entry_type = value
            .get("entry_type")
            .ok_or_else(|| D::Error::missing_field("entry_type"))?
            .as_str()
            .ok_or_else(|| D::Error::custom("entry_type must be a string"))?
            .to_owned();

        macro_rules! route {
            ($variant:ident, $inner:ty) => {{
                let p: $inner = serde_json::from_value(value).map_err(D::Error::custom)?;
                Ok(Self::$variant(p))
            }};
        }

        match entry_type.as_str() {
            "Login" => route!(Login, LoginPayload),
            "Card" => route!(Card, CardPayload),
            "SshKey" => route!(SshKey, SshKeyPayload),
            "ApiKey" => route!(ApiKey, ApiKeyPayload),
            "EnvVars" => route!(EnvVars, EnvVarsPayload),
            "Note" => route!(Note, NotePayload),
            "Document" => route!(Document, DocumentPayload),
            "Identity" => route!(Identity, IdentityPayload),
            "Folder" => route!(Folder, FolderPayload),
            _ => {
                let meta: CommonMeta =
                    serde_json::from_value(value.clone()).map_err(D::Error::custom)?;
                Ok(Self::Unknown(UnknownPayload {
                    meta,
                    unknown_fields: value,
                }))
            }
        }
    }
}
