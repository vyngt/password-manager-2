//! Shared DTOs: entry type, common metadata, and small conversion helpers
//! (timestamps, base64).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use vedge_core::domain::shared::{EntryId, TagId, Timestamp};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryType};

use crate::error::CommandError;

/// `PascalCase` mirror of `vedge_core::EntryType`. Serializes identically to
/// the on-disk form so payload JSON stays interchangeable across layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryTypeDto {
    Login,
    Card,
    SshKey,
    ApiKey,
    EnvVars,
    Note,
    Document,
    Identity,
    Folder,
    #[serde(untagged)]
    Unknown(String),
}

impl From<&EntryType> for EntryTypeDto {
    fn from(v: &EntryType) -> Self {
        match v {
            EntryType::Login => Self::Login,
            EntryType::Card => Self::Card,
            EntryType::SshKey => Self::SshKey,
            EntryType::ApiKey => Self::ApiKey,
            EntryType::EnvVars => Self::EnvVars,
            EntryType::Note => Self::Note,
            EntryType::Document => Self::Document,
            EntryType::Identity => Self::Identity,
            EntryType::Folder => Self::Folder,
            EntryType::Unknown(s) => Self::Unknown(s.clone()),
        }
    }
}

impl From<EntryTypeDto> for EntryType {
    fn from(v: EntryTypeDto) -> Self {
        match v {
            EntryTypeDto::Login => Self::Login,
            EntryTypeDto::Card => Self::Card,
            EntryTypeDto::SshKey => Self::SshKey,
            EntryTypeDto::ApiKey => Self::ApiKey,
            EntryTypeDto::EnvVars => Self::EnvVars,
            EntryTypeDto::Note => Self::Note,
            EntryTypeDto::Document => Self::Document,
            EntryTypeDto::Identity => Self::Identity,
            EntryTypeDto::Folder => Self::Folder,
            EntryTypeDto::Unknown(s) => Self::Unknown(s),
        }
    }
}

/// Mirror of `vedge_core::CommonMeta` with IDs as strings.
///
/// The `notes` field is *not* a secret in the domain (it's stored in the
/// encrypted payload but is considered general metadata), so it crosses
/// as plain `String`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonMetaDto {
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
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

impl From<&CommonMeta> for CommonMetaDto {
    fn from(m: &CommonMeta) -> Self {
        Self {
            name: m.name.clone(),
            entry_type: (&m.entry_type).into(),
            url: m.url.clone(),
            favicon_url: m.favicon_url.clone(),
            tag_ids: m.tag_ids.iter().map(|t| t.as_str().to_owned()).collect(),
            folder_id: m.folder_id.as_ref().map(|f| f.as_str().to_owned()),
            is_favorite: m.is_favorite,
            notes: m.notes.clone(),
        }
    }
}

impl CommonMetaDto {
    /// Convert to a domain `CommonMeta`. `payload_schema` is always set to
    /// the current constant — the frontend does not choose schema versions.
    pub fn into_domain(self) -> CommonMeta {
        CommonMeta {
            name: self.name,
            entry_type: self.entry_type.into(),
            url: self.url,
            favicon_url: self.favicon_url,
            tag_ids: self.tag_ids.into_iter().map(TagId::from_raw).collect(),
            folder_id: self.folder_id.map(EntryId::from_raw),
            is_favorite: self.is_favorite,
            notes: self.notes,
            payload_schema: vedge_core::domain::vault::payloads::CURRENT_PAYLOAD_SCHEMA,
        }
    }
}

// ---- helpers -----------------------------------------------------------------

/// Convert a domain `Timestamp` to an RFC-3339 string with millisecond
/// precision — the format the JS `Date` constructor accepts losslessly.
#[must_use]
pub fn ts_to_string(ts: Timestamp) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Parse an RFC-3339 string into a domain `Timestamp` (UTC). Returns
/// `Invalid` on malformed input.
pub fn ts_from_string(s: &str) -> Result<Timestamp, CommandError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CommandError::Invalid(format!("bad timestamp: {e}")))
}

/// Base64-encode a byte slice using the standard alphabet with padding.
#[must_use]
pub fn b64_encode(bytes: &[u8]) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    STANDARD.encode(bytes)
}

/// Decode a base64 string into bytes. Accepts both standard and URL-safe
/// alphabets with or without padding.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, CommandError> {
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine as _;
    STANDARD
        .decode(s)
        .or_else(|_| URL_SAFE_NO_PAD.decode(s))
        .map_err(|e| CommandError::Invalid(format!("bad base64: {e}")))
}

/// Decode a base64 string and coerce into a fixed-length array.
pub fn b64_decode_fixed<const N: usize>(s: &str) -> Result<[u8; N], CommandError> {
    let v = b64_decode(s)?;
    <[u8; N]>::try_from(v.as_slice()).map_err(|_| {
        CommandError::Invalid(format!("base64 payload must decode to {N} bytes"))
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn entry_type_round_trips_known_variants() {
        for v in [
            EntryType::Login,
            EntryType::Card,
            EntryType::SshKey,
            EntryType::ApiKey,
            EntryType::EnvVars,
            EntryType::Note,
            EntryType::Document,
            EntryType::Identity,
            EntryType::Folder,
        ] {
            let dto: EntryTypeDto = (&v).into();
            let back: EntryType = dto.into();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn entry_type_preserves_unknown() {
        let v = EntryType::Unknown("Passkey".into());
        let dto: EntryTypeDto = (&v).into();
        let back: EntryType = dto.into();
        assert_eq!(back, v);
    }

    #[test]
    fn common_meta_round_trips() {
        let m = CommonMeta {
            name: "github".into(),
            entry_type: EntryType::Login,
            url: Some("https://github.com".into()),
            favicon_url: None,
            tag_ids: vec![TagId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV")],
            folder_id: Some(EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAW")),
            is_favorite: true,
            notes: Some("ok".into()),
            payload_schema: 1,
        };
        let dto: CommonMetaDto = (&m).into();
        let back = dto.into_domain();
        assert_eq!(back.name, m.name);
        assert_eq!(back.entry_type, m.entry_type);
        assert_eq!(back.url, m.url);
        assert_eq!(back.tag_ids, m.tag_ids);
        assert_eq!(back.folder_id, m.folder_id);
        assert_eq!(back.is_favorite, m.is_favorite);
        assert_eq!(back.notes, m.notes);
    }

    #[test]
    fn timestamp_round_trips() {
        let ts = Utc::now();
        let s = ts_to_string(ts);
        let back = ts_from_string(&s).unwrap();
        // Millisecond precision — equal at that resolution.
        assert_eq!(
            back.timestamp_millis(),
            ts.timestamp_millis(),
        );
    }

    #[test]
    fn b64_fixed_rejects_wrong_length() {
        let err = b64_decode_fixed::<32>(&b64_encode(&[0u8; 16])).unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }
}
