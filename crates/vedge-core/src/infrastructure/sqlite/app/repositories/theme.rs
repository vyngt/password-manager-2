use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};

use crate::application::app::ports::ThemeRepository;
use crate::domain::app::entities::Theme;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{StorageError, ThemeId};
use crate::infrastructure::sqlite::app::entities::theme::{ActiveModel, Column, Entity};
use crate::infrastructure::sqlite::app::mappers::theme::{domain_to_model, model_to_domain};

pub struct SqliteThemeRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteThemeRepository {
    #[must_use]
    pub const fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }
}

#[allow(clippy::needless_pass_by_value)] // signature matches `map_err` combinator
fn db_err(e: sea_orm::DbErr) -> AppDbError {
    AppDbError::Storage(StorageError::Database(e.to_string()))
}

#[async_trait]
impl ThemeRepository for SqliteThemeRepository {
    async fn list(&self) -> Result<Vec<Theme>, AppDbError> {
        let rows = Entity::find()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter()
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .collect()
    }

    async fn get(&self, id: &ThemeId) -> Result<Theme, AppDbError> {
        let model = Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::ThemeNotFound(id.clone()))?;
        model_to_domain(model).map_err(AppDbError::from)
    }

    async fn upsert(&self, theme: &Theme) -> Result<(), AppDbError> {
        if let Some(existing) = Entity::find_by_id(theme.id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
        {
            if existing.is_built_in != 0 && !theme.is_built_in {
                return Err(AppDbError::BuiltInThemeImmutable);
            }
        }

        let model = domain_to_model(theme);
        let active = ActiveModel {
            id: ActiveValue::Set(model.id),
            name: ActiveValue::Set(model.name),
            is_built_in: ActiveValue::Set(model.is_built_in),
            root_background: ActiveValue::Set(model.root_background),
            root_foreground: ActiveValue::Set(model.root_foreground),
            root_primary: ActiveValue::Set(model.root_primary),
            danger_base: ActiveValue::Set(model.danger_base),
            warning_base: ActiveValue::Set(model.warning_base),
            success_base: ActiveValue::Set(model.success_base),
            created_at: ActiveValue::Set(model.created_at),
            updated_at: ActiveValue::Set(model.updated_at),
        };

        Entity::insert(active)
            .on_conflict(
                OnConflict::column(Column::Id)
                    .update_columns([
                        Column::Name,
                        Column::IsBuiltIn,
                        Column::RootBackground,
                        Column::RootForeground,
                        Column::RootPrimary,
                        Column::DangerBase,
                        Column::WarningBase,
                        Column::SuccessBase,
                        Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: &ThemeId) -> Result<(), AppDbError> {
        let existing = Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::ThemeNotFound(id.clone()))?;
        if existing.is_built_in != 0 {
            return Err(AppDbError::BuiltInThemeImmutable);
        }
        Entity::delete_by_id(id.as_str().to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }
}
