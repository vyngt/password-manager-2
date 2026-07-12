use serde::{Deserialize, Serialize};

use crate::domain::shared::{EntryId, TagId, Timestamp};
use crate::domain::vault::payloads::entry_type::EntryType;

pub const CURRENT_PAYLOAD_SCHEMA: u32 = 1;

const fn default_payload_schema() -> u32 {
    CURRENT_PAYLOAD_SCHEMA
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonMeta {
    pub name: String,
    pub entry_type: EntryType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,

    #[serde(default)]
    pub tag_ids: Vec<TagId>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<EntryId>,

    #[serde(default)]
    pub is_favorite: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Presentational hint for folders (and, in principle, any entry): a CSS
    /// color string. `None` = use the default folder color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Presentational hint for folders: a curated icon key (resolved to an
    /// `icondata` icon on the frontend). `None` = the default folder icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Manual ordering position within the entry's folder (lower first). `0` for
    /// every entry until it's dragged; ties fall back to name order.
    #[serde(default)]
    pub sort_order: u32,

    /// When this entry's *secret material* last changed (slice 4.3 — password
    /// health). Deliberately **not** `updated_at`, which every metadata mutation
    /// (`set_favorite`, `set_tags`, `set_sort_order`, `move_entry`,
    /// `soft_delete_entry`, `restore_entry`) bumps — starring an entry must not
    /// reset its apparent secret age. Owned **exclusively** by `create_entry` /
    /// `update_entry`; the frontend never supplies it (`common_meta_from_dto`
    /// sets `None`). `None` on pre-4.3 payloads — the scan falls back to history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_changed_at: Option<Timestamp>,

    #[serde(default = "default_payload_schema")]
    pub payload_schema: u32,
}

impl CommonMeta {
    pub fn new(name: impl Into<String>, entry_type: EntryType) -> Self {
        Self {
            name: name.into(),
            entry_type,
            url: None,
            favicon_url: None,
            tag_ids: Vec::new(),
            folder_id: None,
            is_favorite: false,
            notes: None,
            color: None,
            icon: None,
            sort_order: 0,
            secret_changed_at: None,
            payload_schema: CURRENT_PAYLOAD_SCHEMA,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_minimal_meta() {
        let m = CommonMeta::new("github", EntryType::Login);
        let json = serde_json::to_string(&m).unwrap();
        let m2: CommonMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn round_trips_full_meta() {
        let m = CommonMeta {
            name: "GitHub".into(),
            entry_type: EntryType::Login,
            url: Some("https://github.com".into()),
            favicon_url: Some("https://github.com/favicon.ico".into()),
            tag_ids: vec![TagId::from_raw("01H0000000000000000000TAG1")],
            folder_id: Some(EntryId::from_raw("01ARZ3NDEKTSV4RRFFQ69G5FAV")),
            is_favorite: true,
            notes: Some("test notes".into()),
            color: Some("#4f46e5".into()),
            icon: Some("star".into()),
            sort_order: 7,
            secret_changed_at: None,
            payload_schema: 1,
        };
        let json = serde_json::to_string(&m).unwrap();
        let m2: CommonMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn payload_schema_defaults_to_one_when_missing() {
        let json = r#"{"name":"x","entry_type":"Note"}"#;
        let m: CommonMeta = serde_json::from_str(json).unwrap();
        assert_eq!(m.payload_schema, 1);
    }

    #[test]
    fn missing_optionals_omitted_in_output() {
        let m = CommonMeta::new("x", EntryType::Note);
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("\"url\""));
        assert!(!json.contains("\"favicon_url\""));
        assert!(!json.contains("\"folder_id\""));
        assert!(!json.contains("\"notes\""));
    }
}
