use crate::business::domain::entities::theme::{ColorScheme, UpdateColorScheme};
use crate::infra::data::sqlite::entities::theme::color_scheme;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use uuid::Uuid;

pub struct ColorSchemeMapper;

impl ColorSchemeMapper {
    /// Convert database model to domain entity
    pub fn to_domain(data: color_scheme::Model) -> ColorScheme {
        ColorScheme {
            id: data.id,
            name: data.name,
            color_primary: data.color_primary,
            color_secondary: data.color_secondary,
            color_success: data.color_success,
            color_danger: data.color_danger,
            color_warning: data.color_warning,
            color_foreground: data.color_foreground,
            color_background: data.color_background,
        }
    }

    /// Convert domain entity to database active model for creation
    pub fn to_persistence(data: ColorScheme) -> color_scheme::ActiveModel {
        color_scheme::ActiveModel {
            id: Set(data.id),
            name: Set(data.name),
            color_primary: Set(data.color_primary),
            color_secondary: Set(data.color_secondary),
            color_success: Set(data.color_success),
            color_danger: Set(data.color_danger),
            color_warning: Set(data.color_warning),
            color_foreground: Set(data.color_foreground),
            color_background: Set(data.color_background),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
    }

    /// Convert update domain entity to database active model for updates
    pub fn to_update(id: Uuid, data: UpdateColorScheme) -> color_scheme::ActiveModel {
        color_scheme::ActiveModel {
            id: Set(id),
            name: Set(data.name),
            color_primary: Set(data.color_primary),
            color_secondary: Set(data.color_secondary),
            color_success: Set(data.color_success),
            color_danger: Set(data.color_danger),
            color_warning: Set(data.color_warning),
            color_foreground: Set(data.color_foreground),
            color_background: Set(data.color_background),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
    }
}
