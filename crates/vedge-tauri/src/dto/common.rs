//! Conversion layer between `vedge_ipc` wire types and `vedge-core`
//! domain types.
//!
//! The wire types live in `vedge-ipc`; this module adds the domain-aware
//! helpers the orphan rule prevents us from hanging on the foreign DTO
//! types directly.

pub use vedge_ipc::{CommonMetaDto, EntryTypeDto, Timestamp, ts_to_string};

use vedge_core::domain::shared::{EntryId, TagId};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryType};

use crate::error::CommandError;

// ---- EntryTypeDto ↔ EntryType -----------------------------------------------

#[must_use]
pub fn entry_type_to_dto(v: &EntryType) -> EntryTypeDto {
    match v {
        EntryType::Login => EntryTypeDto::Login,
        EntryType::Card => EntryTypeDto::Card,
        EntryType::SshKey => EntryTypeDto::SshKey,
        EntryType::ApiKey => EntryTypeDto::ApiKey,
        EntryType::EnvVars => EntryTypeDto::EnvVars,
        EntryType::Note => EntryTypeDto::Note,
        EntryType::Document => EntryTypeDto::Document,
        EntryType::Identity => EntryTypeDto::Identity,
        EntryType::Folder => EntryTypeDto::Folder,
        EntryType::Unknown(s) => EntryTypeDto::Unknown(s.clone()),
    }
}

#[must_use]
pub fn entry_type_from_dto(v: EntryTypeDto) -> EntryType {
    match v {
        EntryTypeDto::Login => EntryType::Login,
        EntryTypeDto::Card => EntryType::Card,
        EntryTypeDto::SshKey => EntryType::SshKey,
        EntryTypeDto::ApiKey => EntryType::ApiKey,
        EntryTypeDto::EnvVars => EntryType::EnvVars,
        EntryTypeDto::Note => EntryType::Note,
        EntryTypeDto::Document => EntryType::Document,
        EntryTypeDto::Identity => EntryType::Identity,
        EntryTypeDto::Folder => EntryType::Folder,
        EntryTypeDto::Unknown(s) => EntryType::Unknown(s),
    }
}

// ---- CommonMetaDto ↔ CommonMeta ---------------------------------------------

#[must_use]
pub fn common_meta_to_dto(m: &CommonMeta) -> CommonMetaDto {
    CommonMetaDto {
        name: m.name.clone(),
        entry_type: entry_type_to_dto(&m.entry_type),
        url: m.url.clone(),
        favicon_url: m.favicon_url.clone(),
        tag_ids: m.tag_ids.iter().map(|t| t.as_str().to_owned()).collect(),
        folder_id: m.folder_id.as_ref().map(|f| f.as_str().to_owned()),
        is_favorite: m.is_favorite,
        notes: m.notes.clone(),
        color: m.color.clone(),
        icon: m.icon.clone(),
    }
}

/// Convert a `CommonMetaDto` to a domain `CommonMeta`. `payload_schema` is
/// always set to the current constant — the frontend does not choose
/// schema versions.
#[must_use]
pub fn common_meta_from_dto(dto: CommonMetaDto) -> CommonMeta {
    CommonMeta {
        name: dto.name,
        entry_type: entry_type_from_dto(dto.entry_type),
        url: dto.url,
        favicon_url: dto.favicon_url,
        tag_ids: dto.tag_ids.into_iter().map(TagId::from_raw).collect(),
        folder_id: dto.folder_id.map(EntryId::from_raw),
        is_favorite: dto.is_favorite,
        notes: dto.notes,
        color: dto.color,
        icon: dto.icon,
        payload_schema: vedge_core::domain::vault::payloads::CURRENT_PAYLOAD_SCHEMA,
    }
}

// ---- Base64 / timestamp re-exports with CommandError mapping -----------------

/// Base64-encode a byte slice.
#[must_use]
pub fn b64_encode(bytes: &[u8]) -> String {
    vedge_ipc::b64_encode(bytes)
}

/// Decode base64 → bytes. Wire errors map to `CommandError::Invalid`.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, CommandError> {
    vedge_ipc::b64_decode(s).map_err(Into::into)
}

/// Decode base64 → fixed-length array.
pub fn b64_decode_fixed<const N: usize>(s: &str) -> Result<[u8; N], CommandError> {
    vedge_ipc::b64_decode_fixed::<N>(s).map_err(Into::into)
}

/// Parse RFC-3339 → `Timestamp`.
pub fn ts_from_string(s: &str) -> Result<Timestamp, CommandError> {
    vedge_ipc::ts_from_string(s).map_err(Into::into)
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
            let dto = entry_type_to_dto(&v);
            let back = entry_type_from_dto(dto);
            assert_eq!(back, v);
        }
    }

    #[test]
    fn entry_type_preserves_unknown() {
        let v = EntryType::Unknown("Passkey".into());
        let dto = entry_type_to_dto(&v);
        let back = entry_type_from_dto(dto);
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
            color: Some("#4f46e5".into()),
            icon: Some("briefcase".into()),
            payload_schema: 1,
        };
        let dto = common_meta_to_dto(&m);
        let back = common_meta_from_dto(dto);
        assert_eq!(back.name, m.name);
        assert_eq!(back.entry_type, m.entry_type);
        assert_eq!(back.url, m.url);
        assert_eq!(back.tag_ids, m.tag_ids);
        assert_eq!(back.folder_id, m.folder_id);
        assert_eq!(back.is_favorite, m.is_favorite);
        assert_eq!(back.notes, m.notes);
        assert_eq!(back.color, m.color);
        assert_eq!(back.icon, m.icon);
    }

    #[test]
    fn b64_fixed_rejects_wrong_length() {
        let err = b64_decode_fixed::<32>(&b64_encode(&[0u8; 16])).unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }
}
