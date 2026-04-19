//! Tag DTO conversion layer.

pub use vedge_ipc::{CreateTagDto, RenameTagDto, TagMetaDto};

use vedge_core::domain::vault::index::TagMeta;

#[must_use]
pub fn tag_meta_to_dto(t: &TagMeta) -> TagMetaDto {
    TagMetaDto {
        id: t.id.as_str().to_owned(),
        name: t.name.clone(),
        color: t.color.clone(),
        sort_order: t.sort_order,
    }
}
