//! Shared DTOs + wire-format helpers.
//!
//! - `EntryTypeDto` mirrors `vedge_core::EntryType` as serde-only data.
//! - `CommonMetaDto` mirrors `vedge_core::CommonMeta`.
//! - `ts_to_string` / `ts_from_string` round-trip `chrono::DateTime<Utc>`
//!   in RFC-3339 with millisecond precision.
//! - `b64_encode` / `b64_decode` / `b64_decode_fixed` wrap base64 with
//!   `IpcError` mapping.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::IpcError;

pub type Timestamp = DateTime<Utc>;

/// `PascalCase` mirror of `vedge_core::EntryType`. Serializes identically
/// to the on-disk form so payload JSON stays interchangeable across layers.
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

/// Mirror of `vedge_core::CommonMeta` with IDs as strings.
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
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

// ---- helpers ----------------------------------------------------------------

/// Convert a `Timestamp` to an RFC-3339 string with millisecond precision.
#[must_use]
pub fn ts_to_string(ts: Timestamp) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Parse an RFC-3339 string into a `Timestamp` (UTC).
pub fn ts_from_string(s: &str) -> Result<Timestamp, IpcError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| IpcError::BadTimestamp(e.to_string()))
}

/// Base64-encode using the standard alphabet with padding.
#[must_use]
pub fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    STANDARD.encode(bytes)
}

/// Decode a base64 string. Accepts standard and URL-safe alphabets.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, IpcError> {
    use base64::Engine as _;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    STANDARD
        .decode(s)
        .or_else(|_| URL_SAFE_NO_PAD.decode(s))
        .map_err(|e| IpcError::BadBase64(e.to_string()))
}

/// Decode and coerce into a fixed-length array.
pub fn b64_decode_fixed<const N: usize>(s: &str) -> Result<[u8; N], IpcError> {
    let v = b64_decode(s)?;
    let actual = v.len();
    <[u8; N]>::try_from(v.as_slice()).map_err(|_| IpcError::WrongLength { expected: N, actual })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn timestamp_round_trips_millis() {
        use chrono::TimeZone;
        let ts = Utc.with_ymd_and_hms(2026, 4, 19, 12, 0, 0).unwrap();
        let s = ts_to_string(ts);
        let back = ts_from_string(&s).unwrap();
        assert_eq!(back.timestamp_millis(), ts.timestamp_millis());
    }

    #[test]
    fn b64_fixed_rejects_wrong_length() {
        let err = b64_decode_fixed::<32>(&b64_encode(&[0u8; 16])).unwrap_err();
        assert!(matches!(err, IpcError::WrongLength { expected: 32, actual: 16 }));
    }

    #[test]
    fn entry_type_dto_serde_known_variants() {
        let s = serde_json::to_string(&EntryTypeDto::Login).unwrap();
        assert_eq!(s, "\"Login\"");
        let back: EntryTypeDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back, EntryTypeDto::Login);
    }

    #[test]
    fn entry_type_dto_preserves_unknown() {
        let dto = EntryTypeDto::Unknown("Passkey".into());
        let s = serde_json::to_string(&dto).unwrap();
        let back: EntryTypeDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back, dto);
    }
}
