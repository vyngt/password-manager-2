//! Tag wire types. Tags are non-secret: name/color/sort surface in the UI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMetaDto {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagDto {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameTagDto {
    pub tag_id: String,
    pub new_name: String,
}
