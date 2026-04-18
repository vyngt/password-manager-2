use crate::domain::app::entities::Theme;
use crate::domain::shared::{StorageError, ThemeId};
use crate::infrastructure::sqlite::app::entities::theme::Model;

use super::{string_to_ts, ts_to_string};

pub fn model_to_domain(model: Model) -> Result<Theme, StorageError> {
    Ok(Theme {
        id: ThemeId::from_raw(model.id),
        name: model.name,
        is_built_in: model.is_built_in != 0,
        root_background: model.root_background,
        root_foreground: model.root_foreground,
        root_primary: model.root_primary,
        danger_base: model.danger_base,
        warning_base: model.warning_base,
        success_base: model.success_base,
        created_at: string_to_ts(&model.created_at)?,
        updated_at: string_to_ts(&model.updated_at)?,
    })
}

#[must_use] 
pub fn domain_to_model(theme: &Theme) -> Model {
    Model {
        id: theme.id.as_str().to_owned(),
        name: theme.name.clone(),
        is_built_in: i32::from(theme.is_built_in),
        root_background: theme.root_background.clone(),
        root_foreground: theme.root_foreground.clone(),
        root_primary: theme.root_primary.clone(),
        danger_base: theme.danger_base.clone(),
        warning_base: theme.warning_base.clone(),
        success_base: theme.success_base.clone(),
        created_at: ts_to_string(&theme.created_at),
        updated_at: ts_to_string(&theme.updated_at),
    }
}
