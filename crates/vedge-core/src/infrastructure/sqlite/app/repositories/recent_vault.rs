use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::application::app::ports::RecentVaultRepository;
use crate::domain::app::entities::RecentVault;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{StorageError, Timestamp};
use crate::infrastructure::sqlite::app::entities::recent_vault::{
    ActiveModel, Column, Entity,
};
use crate::infrastructure::sqlite::app::mappers::recent_vault::{
    domain_to_model, model_to_domain,
};
use crate::infrastructure::sqlite::app::mappers::ts_to_string;

pub struct SqliteRecentVaultRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteRecentVaultRepository {
    pub fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }
}

fn db_err(e: sea_orm::DbErr) -> AppDbError {
    AppDbError::Storage(StorageError::Database(e.to_string()))
}

#[async_trait]
impl RecentVaultRepository for SqliteRecentVaultRepository {
    async fn list(&self) -> Result<Vec<RecentVault>, AppDbError> {
        let rows = Entity::find().all(self.conn.as_ref()).await.map_err(db_err)?;
        rows.into_iter()
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .collect()
    }

    async fn get(&self, id: &str) -> Result<RecentVault, AppDbError> {
        let model = Entity::find_by_id(id.to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::RecentVaultNotFound(id.to_owned()))?;
        model_to_domain(model).map_err(AppDbError::from)
    }

    async fn upsert(&self, vault: &RecentVault) -> Result<(), AppDbError> {
        let model = domain_to_model(vault);
        let active: ActiveModel = ActiveModel {
            id: ActiveValue::Set(model.id),
            path: ActiveValue::Set(model.path),
            display_name: ActiveValue::Set(model.display_name),
            last_opened: ActiveValue::Set(model.last_opened),
            sort_order: ActiveValue::Set(model.sort_order),
        };
        Entity::insert(active)
            .on_conflict(
                OnConflict::column(Column::Id)
                    .update_columns([
                        Column::Path,
                        Column::DisplayName,
                        Column::LastOpened,
                        Column::SortOrder,
                    ])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppDbError> {
        let res = Entity::delete_by_id(id.to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(AppDbError::RecentVaultNotFound(id.to_owned()));
        }
        Ok(())
    }

    async fn touch_last_opened(&self, id: &str, when: Timestamp) -> Result<(), AppDbError> {
        let model = Entity::find_by_id(id.to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::RecentVaultNotFound(id.to_owned()))?;
        let mut active: ActiveModel = model.into();
        active.last_opened = ActiveValue::Set(Some(ts_to_string(&when)));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }
}

