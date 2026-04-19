use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;

use crate::application::app::ports::AppSettingRepository;
use crate::domain::app::entities::AppSetting;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{StorageError, Timestamp};
use crate::infrastructure::sqlite::app::entities::app_setting::{ActiveModel, Column, Entity};
use crate::infrastructure::sqlite::app::mappers::app_setting::{domain_to_model, model_to_domain};

pub struct SqliteAppSettingRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteAppSettingRepository {
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
impl AppSettingRepository for SqliteAppSettingRepository {
    async fn get(&self, key: &str) -> Result<Option<AppSetting>, AppDbError> {
        let model = Entity::find_by_id(key.to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        model
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .transpose()
    }

    async fn set(&self, key: &str, value: Value, when: Timestamp) -> Result<(), AppDbError> {
        let setting = AppSetting {
            key: key.to_owned(),
            value,
            updated_at: when,
        };
        let model = domain_to_model(&setting).map_err(AppDbError::from)?;
        let active = ActiveModel {
            key: ActiveValue::Set(model.key),
            value: ActiveValue::Set(model.value),
            updated_at: ActiveValue::Set(model.updated_at),
        };
        Entity::insert(active)
            .on_conflict(
                OnConflict::column(Column::Key)
                    .update_columns([Column::Value, Column::UpdatedAt])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), AppDbError> {
        let res = Entity::delete_by_id(key.to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(AppDbError::SettingNotFound(key.to_owned()));
        }
        Ok(())
    }

    async fn list_prefix(&self, prefix: &str) -> Result<Vec<AppSetting>, AppDbError> {
        let pattern = format!("{prefix}%");
        let rows = Entity::find()
            .filter(Column::Key.like(pattern))
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter()
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .collect()
    }
}
