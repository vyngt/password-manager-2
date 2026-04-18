use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::application::app::ports::KnownDeviceRepository;
use crate::domain::app::entities::KnownDevice;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{DeviceId, StorageError, Timestamp};
use crate::infrastructure::sqlite::app::entities::known_device::{
    ActiveModel, Column, Entity,
};
use crate::infrastructure::sqlite::app::mappers::known_device::{
    domain_to_model, model_to_domain,
};
use crate::infrastructure::sqlite::app::mappers::ts_to_string;

pub struct SqliteKnownDeviceRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteKnownDeviceRepository {
    pub fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }
}

fn db_err(e: sea_orm::DbErr) -> AppDbError {
    AppDbError::Storage(StorageError::Database(e.to_string()))
}

#[async_trait]
impl KnownDeviceRepository for SqliteKnownDeviceRepository {
    async fn list(&self) -> Result<Vec<KnownDevice>, AppDbError> {
        let rows = Entity::find().all(self.conn.as_ref()).await.map_err(db_err)?;
        rows.into_iter()
            .map(|m| model_to_domain(m).map_err(AppDbError::from))
            .collect()
    }

    async fn upsert(&self, device: &KnownDevice) -> Result<(), AppDbError> {
        let model = domain_to_model(device);
        let active = ActiveModel {
            device_id: ActiveValue::Set(model.device_id),
            display_name: ActiveValue::Set(model.display_name),
            public_key: ActiveValue::Set(model.public_key),
            first_seen: ActiveValue::Set(model.first_seen),
            last_seen: ActiveValue::Set(model.last_seen),
        };
        Entity::insert(active)
            .on_conflict(
                OnConflict::column(Column::DeviceId)
                    .update_columns([
                        Column::DisplayName,
                        Column::PublicKey,
                        Column::LastSeen,
                    ])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn touch_last_seen(
        &self,
        id: &DeviceId,
        when: Timestamp,
    ) -> Result<(), AppDbError> {
        let model = Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| AppDbError::DeviceNotFound(id.clone()))?;
        let mut active: ActiveModel = model.into();
        active.last_seen = ActiveValue::Set(Some(ts_to_string(&when)));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: &DeviceId) -> Result<(), AppDbError> {
        let res = Entity::delete_by_id(id.as_str().to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(AppDbError::DeviceNotFound(id.clone()));
        }
        Ok(())
    }
}
