use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::application::app::ports::ExtensionSessionRepository;
use crate::domain::app::entities::ExtensionSession;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{SessionId, StorageError, Timestamp};
use crate::infrastructure::sqlite::app::entities::extension_session::{
    ActiveModel, Column, Entity,
};
use crate::infrastructure::sqlite::app::mappers::extension_session::{
    domain_to_model, model_to_domain,
};
use crate::infrastructure::sqlite::app::mappers::ts_to_string;

pub struct SqliteExtensionSessionRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteExtensionSessionRepository {
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
impl ExtensionSessionRepository for SqliteExtensionSessionRepository {
    async fn list(&self) -> Result<Vec<ExtensionSession>, AppDbError> {
        let rows = Entity::find()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter()
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .collect()
    }

    async fn upsert(&self, session: &ExtensionSession) -> Result<(), AppDbError> {
        let model = domain_to_model(session);
        let active = ActiveModel {
            session_id: ActiveValue::Set(model.session_id),
            browser: ActiveValue::Set(model.browser),
            profile_name: ActiveValue::Set(model.profile_name),
            session_key: ActiveValue::Set(model.session_key),
            created_at: ActiveValue::Set(model.created_at),
            last_active_at: ActiveValue::Set(model.last_active_at),
        };
        Entity::insert(active)
            .on_conflict(
                OnConflict::column(Column::SessionId)
                    .update_columns([
                        Column::Browser,
                        Column::ProfileName,
                        Column::SessionKey,
                        Column::LastActiveAt,
                    ])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn touch_last_active(&self, id: &SessionId, when: Timestamp) -> Result<(), AppDbError> {
        let model = Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::ExtensionSessionNotFound(id.clone()))?;
        let mut active: ActiveModel = model.into();
        active.last_active_at = ActiveValue::Set(Some(ts_to_string(&when)));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: &SessionId) -> Result<(), AppDbError> {
        let res = Entity::delete_by_id(id.as_str().to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(AppDbError::ExtensionSessionNotFound(id.clone()));
        }
        Ok(())
    }
}
