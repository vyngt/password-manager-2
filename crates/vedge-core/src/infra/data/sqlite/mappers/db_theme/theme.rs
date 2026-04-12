use crate::business::domain::entities::theme::{Theme, UpdateTheme};
use crate::infra::data::sqlite::entities::theme::{color_scheme, theme};
use sea_orm::ActiveValue::Set;
use uuid::Uuid;

pub struct ThemeMapper;

impl ThemeMapper {
    /// Convert database model to domain entity
    pub fn to_domain(the: theme::Model, color_scheme: color_scheme::Model) -> Theme {
        Theme {
            id: the.id,
            color_scheme: super::ColorSchemeMapper::to_domain(color_scheme),
        }
    }

    /// Convert domain entity to database active model for creation
    pub fn to_persistence(data: Theme) -> theme::ActiveModel {
        theme::ActiveModel {
            id: Set(data.id),
            color_scheme_id: Set(Some(data.color_scheme.id)),
        }
    }

    /// Convert update domain entity to database active model for updates
    pub fn to_update(id: Uuid, data: UpdateTheme) -> theme::ActiveModel {
        theme::ActiveModel {
            id: Set(id),
            color_scheme_id: Set(Some(data.color_scheme_id)),
        }
    }
}
