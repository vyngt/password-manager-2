use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, Order,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::application::vault::ports::VaultRepository;
use crate::domain::shared::{EntryId, StorageError, TagId, Timestamp};
use crate::domain::vault::entities::{AuditEvent, EntryHistoryRow, EntryRow, TagRow, VaultConfig};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::sqlite::vault::entities::audit_log::{
    self as audit_entity, Column as AuditCol,
};
use crate::infrastructure::sqlite::vault::entities::entry::{
    self as entry_entity, Column as EntryCol,
};
use crate::infrastructure::sqlite::vault::entities::entry_history::{
    self as history_entity, Column as HistoryCol,
};
use crate::infrastructure::sqlite::vault::entities::tag::{self as tag_entity};
use crate::infrastructure::sqlite::vault::entities::vault_config::{
    self as config_entity, Column as ConfigCol,
};
use crate::infrastructure::sqlite::vault::mappers::{
    audit_log as audit_map, entry as entry_map, entry_history as history_map, tag as tag_map,
    ts_to_string, vault_config as config_map,
};

pub struct SqliteVaultRepository {
    conn: Arc<DatabaseConnection>,
}

impl SqliteVaultRepository {
    #[must_use]
    pub const fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }
}

#[allow(clippy::needless_pass_by_value)] // signature matches `map_err` combinator
fn db_err(e: sea_orm::DbErr) -> VaultError {
    VaultError::Storage(StorageError::Database(e.to_string()))
}

#[async_trait]
impl VaultRepository for SqliteVaultRepository {
    // ---- vault_config -------------------------------------------------------

    async fn load_config(&self) -> Result<VaultConfig, VaultError> {
        let model = config_entity::Entity::find()
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or(VaultError::ConfigMissing)?;
        config_map::model_to_domain(model)
    }

    async fn save_config(&self, config: &VaultConfig) -> Result<(), VaultError> {
        let model = config_map::domain_to_model(config)?;
        let active = config_entity::ActiveModel {
            id: ActiveValue::Set(model.id),
            magic: ActiveValue::Set(model.magic),
            schema_version: ActiveValue::Set(model.schema_version),
            vault_salt: ActiveValue::Set(model.vault_salt),
            kdf_params: ActiveValue::Set(model.kdf_params),
            verify_hash: ActiveValue::Set(model.verify_hash),
            preferred_cipher_suite: ActiveValue::Set(model.preferred_cipher_suite),
            trash_retention_days: ActiveValue::Set(model.trash_retention_days),
            audit_retention_days: ActiveValue::Set(model.audit_retention_days),
            created_at: ActiveValue::Set(model.created_at),
            last_unlocked_at: ActiveValue::Set(model.last_unlocked_at),
        };

        config_entity::Entity::insert(active)
            .on_conflict(
                OnConflict::column(ConfigCol::Id)
                    .update_columns([
                        ConfigCol::Magic,
                        ConfigCol::SchemaVersion,
                        ConfigCol::VaultSalt,
                        ConfigCol::KdfParams,
                        ConfigCol::VerifyHash,
                        ConfigCol::PreferredCipherSuite,
                        ConfigCol::TrashRetentionDays,
                        ConfigCol::AuditRetentionDays,
                        ConfigCol::CreatedAt,
                        ConfigCol::LastUnlockedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    // ---- entries ------------------------------------------------------------

    async fn insert_entry(&self, row: &EntryRow) -> Result<(), VaultError> {
        let model = entry_map::domain_to_model(row);
        let active: entry_entity::ActiveModel = model.into();
        entry_entity::Entity::insert(active)
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn update_entry(&self, row: &EntryRow) -> Result<(), VaultError> {
        let existing = entry_entity::Entity::find_by_id(row.id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::EntryNotFound(row.id.clone()))?;

        let model = entry_map::domain_to_model(row);
        let mut active: entry_entity::ActiveModel = existing.into();
        active.version = ActiveValue::Set(model.version);
        active.cipher_suite = ActiveValue::Set(model.cipher_suite);
        active.dek_wrapped = ActiveValue::Set(model.dek_wrapped);
        active.nonce = ActiveValue::Set(model.nonce);
        active.ciphertext = ActiveValue::Set(model.ciphertext);
        active.updated_at = ActiveValue::Set(model.updated_at);
        active.accessed_at = ActiveValue::Set(model.accessed_at);
        active.is_trashed = ActiveValue::Set(model.is_trashed);
        active.trashed_at = ActiveValue::Set(model.trashed_at);
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn get_entry(&self, id: &EntryId) -> Result<EntryRow, VaultError> {
        let model = entry_entity::Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::EntryNotFound(id.clone()))?;
        entry_map::model_to_domain(model)
    }

    async fn all_entries(&self) -> Result<Vec<EntryRow>, VaultError> {
        let rows = entry_entity::Entity::find()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter().map(entry_map::model_to_domain).collect()
    }

    async fn soft_delete_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError> {
        let model = entry_entity::Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::EntryNotFound(id.clone()))?;
        let mut active: entry_entity::ActiveModel = model.into();
        active.is_trashed = ActiveValue::Set(1);
        active.trashed_at = ActiveValue::Set(Some(ts_to_string(&when)));
        active.updated_at = ActiveValue::Set(ts_to_string(&when));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn restore_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError> {
        let model = entry_entity::Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::EntryNotFound(id.clone()))?;
        let mut active: entry_entity::ActiveModel = model.into();
        active.is_trashed = ActiveValue::Set(0);
        active.trashed_at = ActiveValue::Set(None);
        active.updated_at = ActiveValue::Set(ts_to_string(&when));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn hard_delete_entry(&self, id: &EntryId) -> Result<(), VaultError> {
        let res = entry_entity::Entity::delete_by_id(id.as_str().to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(VaultError::EntryNotFound(id.clone()));
        }
        Ok(())
    }

    async fn hard_delete_trashed_before(&self, cutoff: Timestamp) -> Result<u64, VaultError> {
        let cutoff_str = ts_to_string(&cutoff);
        // Cascade the version history of the entries we're about to purge — the
        // schema has no FK cascade, and this bulk path bypasses the
        // `hard_delete_entry` use case that would otherwise do it.
        let ids: Vec<String> = entry_entity::Entity::find()
            .select_only()
            .column(EntryCol::Id)
            .filter(EntryCol::IsTrashed.eq(1))
            .filter(EntryCol::TrashedAt.lt(cutoff_str.as_str()))
            .into_tuple()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if !ids.is_empty() {
            history_entity::Entity::delete_many()
                .filter(HistoryCol::EntryId.is_in(ids))
                .exec(self.conn.as_ref())
                .await
                .map_err(db_err)?;
        }
        let res = entry_entity::Entity::delete_many()
            .filter(EntryCol::IsTrashed.eq(1))
            .filter(EntryCol::TrashedAt.lt(cutoff_str.as_str()))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }

    async fn update_accessed_at(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError> {
        let model = entry_entity::Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::EntryNotFound(id.clone()))?;
        let mut active: entry_entity::ActiveModel = model.into();
        active.accessed_at = ActiveValue::Set(Some(ts_to_string(&when)));
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    // ---- entry_history ------------------------------------------------------

    async fn insert_history(&self, row: &EntryHistoryRow) -> Result<(), VaultError> {
        let model = history_map::domain_to_model(row);
        let active: history_entity::ActiveModel = model.into();
        history_entity::Entity::insert(active)
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn list_history(&self, entry_id: &EntryId) -> Result<Vec<EntryHistoryRow>, VaultError> {
        let rows = history_entity::Entity::find()
            .filter(HistoryCol::EntryId.eq(entry_id.as_str()))
            .order_by(HistoryCol::Version, Order::Desc)
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter().map(history_map::model_to_domain).collect()
    }

    async fn get_history(&self, id: &str) -> Result<EntryHistoryRow, VaultError> {
        let model = history_entity::Entity::find_by_id(id.to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::HistoryNotFound(id.to_owned()))?;
        history_map::model_to_domain(model)
    }

    async fn delete_history_for_entry(&self, entry_id: &EntryId) -> Result<u64, VaultError> {
        let res = history_entity::Entity::delete_many()
            .filter(HistoryCol::EntryId.eq(entry_id.as_str()))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }

    async fn prune_history_keep(&self, entry_id: &EntryId, keep: usize) -> Result<u64, VaultError> {
        // Ids newest-first; the ones past `keep` are the oldest and get dropped.
        let ids: Vec<String> = history_entity::Entity::find()
            .select_only()
            .column(HistoryCol::Id)
            .filter(HistoryCol::EntryId.eq(entry_id.as_str()))
            .order_by(HistoryCol::Version, Order::Desc)
            .into_tuple()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        let stale: Vec<String> = ids.into_iter().skip(keep).collect();
        if stale.is_empty() {
            return Ok(0);
        }
        let res = history_entity::Entity::delete_many()
            .filter(HistoryCol::Id.is_in(stale))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }

    // ---- tags ---------------------------------------------------------------

    async fn insert_tag(&self, row: &TagRow) -> Result<(), VaultError> {
        let model = tag_map::domain_to_model(row);
        let active: tag_entity::ActiveModel = model.into();
        tag_entity::Entity::insert(active)
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn update_tag(&self, row: &TagRow) -> Result<(), VaultError> {
        let existing = tag_entity::Entity::find_by_id(row.id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::TagNotFound(row.id.clone()))?;
        let model = tag_map::domain_to_model(row);
        let mut active: tag_entity::ActiveModel = existing.into();
        active.nonce = ActiveValue::Set(model.nonce);
        active.ciphertext = ActiveValue::Set(model.ciphertext);
        active.updated_at = ActiveValue::Set(model.updated_at);
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        Ok(())
    }

    async fn get_tag(&self, id: &TagId) -> Result<TagRow, VaultError> {
        let model = tag_entity::Entity::find_by_id(id.as_str().to_owned())
            .one(self.conn.as_ref())
            .await
            .map_err(db_err)?
            .ok_or_else(|| VaultError::TagNotFound(id.clone()))?;
        tag_map::model_to_domain(model)
    }

    async fn all_tags(&self) -> Result<Vec<TagRow>, VaultError> {
        let rows = tag_entity::Entity::find()
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter().map(tag_map::model_to_domain).collect()
    }

    async fn delete_tag(&self, id: &TagId) -> Result<(), VaultError> {
        let res = tag_entity::Entity::delete_by_id(id.as_str().to_owned())
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        if res.rows_affected == 0 {
            return Err(VaultError::TagNotFound(id.clone()));
        }
        Ok(())
    }

    // ---- audit_log ----------------------------------------------------------

    async fn append_audit(&self, event: &AuditEvent) -> Result<(), VaultError> {
        let model = audit_map::domain_to_model(event);
        let active: audit_entity::ActiveModel = model.into();
        audit_entity::Entity::insert(active)
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn recent_audit(&self, limit: u32) -> Result<Vec<AuditEvent>, VaultError> {
        let rows = audit_entity::Entity::find()
            .order_by(AuditCol::OccurredAt, Order::Desc)
            .limit(u64::from(limit))
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter().map(audit_map::model_to_domain).collect()
    }

    async fn audit_by_entry(
        &self,
        entry_id: &EntryId,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, VaultError> {
        let rows = audit_entity::Entity::find()
            .filter(AuditCol::EntryId.eq(entry_id.as_str()))
            .order_by(AuditCol::OccurredAt, Order::Desc)
            .limit(u64::from(limit))
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        rows.into_iter().map(audit_map::model_to_domain).collect()
    }

    async fn delete_audit_before(&self, cutoff: Timestamp) -> Result<u64, VaultError> {
        let cutoff_str = ts_to_string(&cutoff);
        let res = audit_entity::Entity::delete_many()
            .filter(AuditCol::OccurredAt.lt(cutoff_str))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(res.rows_affected)
    }

    async fn rewrap_all_deks(
        &self,
        updates: &[(EntryId, [u8; 40])],
        new_config: &VaultConfig,
    ) -> Result<(), VaultError> {
        let config_model = config_map::domain_to_model(new_config)?;

        let txn = self.conn.begin().await.map_err(db_err)?;

        for (entry_id, dek_wrapped) in updates {
            let active = entry_entity::ActiveModel {
                id: ActiveValue::Unchanged(entry_id.as_str().to_owned()),
                dek_wrapped: ActiveValue::Set(dek_wrapped.to_vec()),
                ..Default::default()
            };
            entry_entity::Entity::update(active)
                .exec(&txn)
                .await
                .map_err(db_err)?;
        }

        let config_active = config_entity::ActiveModel {
            id: ActiveValue::Set(config_model.id.clone()),
            magic: ActiveValue::Set(config_model.magic),
            schema_version: ActiveValue::Set(config_model.schema_version),
            vault_salt: ActiveValue::Set(config_model.vault_salt),
            kdf_params: ActiveValue::Set(config_model.kdf_params),
            verify_hash: ActiveValue::Set(config_model.verify_hash),
            preferred_cipher_suite: ActiveValue::Set(config_model.preferred_cipher_suite),
            trash_retention_days: ActiveValue::Set(config_model.trash_retention_days),
            audit_retention_days: ActiveValue::Set(config_model.audit_retention_days),
            created_at: ActiveValue::Set(config_model.created_at),
            last_unlocked_at: ActiveValue::Set(config_model.last_unlocked_at),
        };
        config_entity::Entity::insert(config_active)
            .on_conflict(
                OnConflict::column(ConfigCol::Id)
                    .update_columns([
                        ConfigCol::Magic,
                        ConfigCol::SchemaVersion,
                        ConfigCol::VaultSalt,
                        ConfigCol::KdfParams,
                        ConfigCol::VerifyHash,
                        ConfigCol::PreferredCipherSuite,
                        ConfigCol::TrashRetentionDays,
                        ConfigCol::AuditRetentionDays,
                        ConfigCol::CreatedAt,
                        ConfigCol::LastUnlockedAt,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(db_err)?;

        txn.commit().await.map_err(db_err)?;
        Ok(())
    }
}
