//! Tag DTOs. Tags are non-secret — `TagPayload` is encrypted at rest under
//! the KEK, but the display name, color, and sort order all surface in the
//! UI, so they cross as plain strings.

use serde::{Deserialize, Serialize};

use vedge_core::domain::vault::index::TagMeta;

/// Read-side projection of a tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMetaDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    pub sort_order: u32,
}

impl From<&TagMeta> for TagMetaDto {
    fn from(t: &TagMeta) -> Self {
        Self {
            id: t.id.as_str().to_owned(),
            name: t.name.clone(),
            color: t.color.clone(),
            sort_order: t.sort_order,
        }
    }
}

/// Input shape for `create_tag`. The domain's `normalize_tag_name`
/// trims / lowercases / collapses whitespace before write — the DTO is
/// the user-facing form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagDto {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// Input shape for `rename_tag`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameTagDto {
    pub tag_id: String,
    pub new_name: String,
}
