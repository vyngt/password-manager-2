use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::application::vault::ports::VaultRepository;
use crate::domain::shared::{EntryId, StorageError, TagId, Timestamp};
use crate::domain::vault::entities::{
    AuditAction, AuditEvent, AuditPage, AuditQuery, EntryHistoryRow, EntryRow, TagRow, VaultConfig,
};
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

/// Advance the rollback commit counter by one (slice 5.2c).
///
/// Called at the **tail** of every content-mutating write so a crash between the
/// write and the bump can only *under*-count (a spurious rollback warning at worst),
/// never over-count (which would mask a real rollback). The `+ 1` is evaluated
/// SQL-side to sidestep the `arithmetic_side_effects` lint, and is `i64` so it will
/// not overflow in any realistic vault lifetime. Generic over the connection so it
/// runs on both the bare `self.conn` and the `rewrap_all_deks` transaction.
async fn bump_commit_counter<C: ConnectionTrait>(conn: &C) -> Result<(), VaultError> {
    config_entity::Entity::update_many()
        .col_expr(
            ConfigCol::CommitCounter,
            Expr::col(ConfigCol::CommitCounter).add(1),
        )
        .filter(ConfigCol::Id.eq("default"))
        .exec(conn)
        .await
        .map_err(db_err)?;
    Ok(())
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
            vault_uuid: ActiveValue::Set(model.vault_uuid),
            // Set on INSERT (a fresh vault seeds 0); on UPDATE it is preserved by
            // OMITTING it from `update_columns` below — else an unlock's `save_config`
            // would clobber the live counter with the stale value read at unlock. The
            // column is advanced ONLY by `bump_commit_counter` + restore's re-baseline.
            commit_counter: ActiveValue::Set(model.commit_counter),
            backup_dir: ActiveValue::Set(model.backup_dir),
            backup_keep_count: ActiveValue::Set(model.backup_keep_count),
            // Written by `touch_last_snapshot_at` / `touch_last_backup_at` (slice 5.2.1) and,
            // like `commit_counter`, OMITTED from `update_columns` so a stale `save_config`
            // cannot clobber a timestamp written after this config was read.
            last_snapshot_at: ActiveValue::Set(model.last_snapshot_at),
            last_backup_at: ActiveValue::Set(model.last_backup_at),
            // Set on INSERT (a fresh vault seeds NULL — recovery off); on UPDATE it is
            // preserved by OMITTING it from `update_columns` below, so a generic
            // `save_config` (e.g. an unlock stamping `last_unlocked_at`) can never
            // clobber or resurrect an enrolled slot. Written ONLY by `set_recovery_slot`
            // / `clear_recovery_slot`, and nulled in-txn by `rewrap_all_deks` on a KEK
            // change. Slice 5.7.
            recovery_slot: ActiveValue::Set(model.recovery_slot),
            // Set on INSERT (a fresh vault seeds NULL); on UPDATE preserved by OMITTING them
            // from `update_columns`, so a generic `save_config` can never clobber the
            // credential age. Written by the credential paths via `rewrap_all_deks` + the
            // targeted `touch_last_password_change_at` / `touch_last_secret_key_rotation_at`.
            // Slice 5.9.
            last_password_change_at: ActiveValue::Set(model.last_password_change_at),
            last_secret_key_rotation_at: ActiveValue::Set(model.last_secret_key_rotation_at),
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
                        ConfigCol::VaultUuid,
                        ConfigCol::BackupDir,
                        ConfigCol::BackupKeepCount,
                        // NOT ConfigCol::CommitCounter / LastSnapshotAt / LastBackupAt /
                        // RecoverySlot / LastPasswordChangeAt / LastSecretKeyRotationAt —
                        // see the field comments above.
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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
        // 🔴 `dek_wrapped` MUST ride the update (slice 5.6.0): the at-unlock tag
        // migration re-encrypts under a fresh DEK and calls this — dropping the DEK
        // here would persist DEK-ciphertext with a NULL DEK, unreadable by either path.
        active.dek_wrapped = ActiveValue::Set(model.dek_wrapped);
        active.updated_at = ActiveValue::Set(model.updated_at);
        active.update(self.conn.as_ref()).await.map_err(db_err)?;
        bump_commit_counter(self.conn.as_ref()).await?;
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
        bump_commit_counter(self.conn.as_ref()).await?;
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

    async fn query_audit(&self, q: &AuditQuery) -> Result<AuditPage, VaultError> {
        // All predicates are applied in SQL — filtering the *page* in memory
        // would break pagination (100 rows filtered to 3 is not a page of 3).
        let mut cond = Condition::all();
        if let Some(id) = &q.entry_id {
            cond = cond.add(AuditCol::EntryId.eq(id.as_str()));
        }
        if !q.actions.is_empty() {
            let names: Vec<&str> = q.actions.iter().map(AuditAction::as_str).collect();
            cond = cond.add(AuditCol::Action.is_in(names));
        }
        // Lexical string comparison == chronological because `occurred_at` is
        // written exclusively through `format_rfc3339_millis` (fixed-width
        // `…T…:…:….000Z`); see `domain/shared/timestamps.rs`. `since` inclusive,
        // `until` exclusive.
        if let Some(s) = &q.since {
            cond = cond.add(AuditCol::OccurredAt.gte(ts_to_string(s)));
        }
        if let Some(u) = &q.until {
            cond = cond.add(AuditCol::OccurredAt.lt(ts_to_string(u)));
        }

        let total = audit_entity::Entity::find()
            .filter(cond.clone())
            .count(self.conn.as_ref())
            .await
            .map_err(db_err)?;

        let rows = audit_entity::Entity::find()
            .filter(cond)
            .order_by(AuditCol::OccurredAt, Order::Desc)
            // Stable tiebreak: `occurred_at` is millisecond-precision, so bulk
            // writes (empty-trash cascade, bulk-generate, tag ops) routinely
            // collide within one ms. Without a unique secondary key, SQLite's
            // order among ties is unspecified and can differ between the count
            // and the paged reads → a colliding row shows on two pages or is
            // skipped. The PK is a ULID (monotonic), so `id DESC` also stays
            // chronological within a tie group.
            .order_by(AuditCol::Id, Order::Desc)
            .limit(u64::from(q.limit))
            .offset(u64::from(q.offset))
            .all(self.conn.as_ref())
            .await
            .map_err(db_err)?;

        // LENIENT mapper: a row whose `action` string this build cannot parse is
        // skipped + warned, not propagated. 4.2/4.3 add action variants, so a
        // downgraded build will meet rows it can't parse — degrade one row
        // instead of failing the page. (`total` counts skipped rows; documented.)
        let events = rows
            .into_iter()
            .filter_map(|m| match audit_map::model_to_domain(m) {
                Ok(e) => Some(e),
                Err(e) => {
                    tracing::warn!(error = %e, "skipping unparseable audit row");
                    None
                }
            })
            .collect();

        Ok(AuditPage { events, total })
    }

    async fn rewrap_all_deks(
        &self,
        updates: &[(EntryId, [u8; 40])],
        tag_updates: &[TagRow],
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

        // Tags (slice 5.6.0): each carries a re-wrapped or migrated per-row DEK. A
        // re-wrap leaves `nonce`/`ciphertext` unchanged; a migrated legacy tag replaces
        // all three. Written in the SAME txn as the entries + config, so `verify_hash`
        // never advances past a tag left under the old KEK.
        for tag in tag_updates {
            let model = tag_map::domain_to_model(tag);
            let active = tag_entity::ActiveModel {
                id: ActiveValue::Unchanged(model.id),
                nonce: ActiveValue::Set(model.nonce),
                ciphertext: ActiveValue::Set(model.ciphertext),
                dek_wrapped: ActiveValue::Set(model.dek_wrapped),
                updated_at: ActiveValue::Set(model.updated_at),
                ..Default::default()
            };
            tag_entity::Entity::update(active)
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
            vault_uuid: ActiveValue::Set(config_model.vault_uuid),
            // Preserved on UPDATE (omitted from `update_columns`) — the bump below
            // is what advances it inside this same txn. See `save_config`'s note.
            commit_counter: ActiveValue::Set(config_model.commit_counter),
            backup_dir: ActiveValue::Set(config_model.backup_dir),
            backup_keep_count: ActiveValue::Set(config_model.backup_keep_count),
            last_snapshot_at: ActiveValue::Set(config_model.last_snapshot_at),
            last_backup_at: ActiveValue::Set(config_model.last_backup_at),
            recovery_slot: ActiveValue::Set(config_model.recovery_slot),
            last_password_change_at: ActiveValue::Set(config_model.last_password_change_at),
            last_secret_key_rotation_at: ActiveValue::Set(config_model.last_secret_key_rotation_at),
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
                        ConfigCol::VaultUuid,
                        ConfigCol::BackupDir,
                        ConfigCol::BackupKeepCount,
                        // 🔴 RecoverySlot IS updated here (slice 5.7 ③, C1) — UNLIKE
                        // `save_config`. A KEK change rewraps every DEK; the slot wraps
                        // the OLD KEK, so `change_password`/`rewrap_one` set
                        // `new_config.recovery_slot = None` and this writes that null in
                        // the SAME txn. Dropping it here would leave a slot that unwraps a
                        // dead KEK — a silent brick at the recovery moment.
                        ConfigCol::RecoverySlot,
                        // 🔴 Credential-age stamps (slice 5.9 ③) ARE updated here, like
                        // `RecoverySlot` — `change_password` sets them on `new_config` and this
                        // persists them in the SAME rewrap txn (`save_config` omits them, so the
                        // rewrap is the only way they ride a full-config upsert).
                        ConfigCol::LastPasswordChangeAt,
                        ConfigCol::LastSecretKeyRotationAt,
                        // NOT ConfigCol::CommitCounter (bumped in-txn just below) /
                        // LastSnapshotAt / LastBackupAt (targeted-update columns).
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(db_err)?;

        // Bump inside the same txn (config upsert deliberately omits CommitCounter
        // from its update_columns, so the bump is what advances it here).
        bump_commit_counter(&txn).await?;

        txn.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn vacuum_into(&self, dest: &Path) -> Result<(), VaultError> {
        // Decision ①: live-safe `VACUUM INTO` through the existing sea-orm
        // connection. It CANNOT run in a transaction (bare `execute_unprepared`)
        // and FAILS if `dest` exists (the caller vacuums into a fresh temp path).
        // No bound params on this statement form; `dest` is an app-controlled temp
        // path, single-quote-escaped into the SQL string literal.
        let path = dest.to_string_lossy().replace('\'', "''");
        self.conn
            .execute_unprepared(&format!("VACUUM INTO '{path}'"))
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn close(&self) {
        // Close the sqlx pool via a shared ref (no ownership needed) — `close().await`
        // returns only once every connection is released, so the OS file handle on the `.vdb`
        // is gone when this returns. `get_sqlite_connection_pool` is valid because every vault
        // connection is SQLite (VaultDbConnection::open).
        self.conn.get_sqlite_connection_pool().close().await;
    }

    async fn touch_last_snapshot_at(&self, at: Timestamp) -> Result<(), VaultError> {
        config_entity::Entity::update_many()
            .col_expr(ConfigCol::LastSnapshotAt, Expr::value(ts_to_string(&at)))
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn touch_last_backup_at(&self, at: Timestamp) -> Result<(), VaultError> {
        config_entity::Entity::update_many()
            .col_expr(ConfigCol::LastBackupAt, Expr::value(ts_to_string(&at)))
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn set_recovery_slot(&self, slot: &[u8; 40]) -> Result<(), VaultError> {
        // Targeted single-column write (slice 5.7), like `touch_last_backup_at` — so it
        // never rides a full-config upsert and can't be clobbered by a stale `save_config`.
        config_entity::Entity::update_many()
            .col_expr(ConfigCol::RecoverySlot, Expr::value(slot.to_vec()))
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn clear_recovery_slot(&self) -> Result<(), VaultError> {
        // Null the slot (revoke, or the null-in-snapshot at capture — slice 5.7 ④⑥).
        config_entity::Entity::update_many()
            .col_expr(
                ConfigCol::RecoverySlot,
                Expr::value(Option::<Vec<u8>>::None),
            )
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn touch_last_password_change_at(&self, at: Timestamp) -> Result<(), VaultError> {
        // Targeted write (slice 5.9), like `touch_last_snapshot_at` — the credential-age
        // columns are omitted from `save_config`, so `rekey_vault` (which persists via
        // `save_config` on the staged repo) stamps them this way instead.
        config_entity::Entity::update_many()
            .col_expr(
                ConfigCol::LastPasswordChangeAt,
                Expr::value(ts_to_string(&at)),
            )
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn touch_last_secret_key_rotation_at(&self, at: Timestamp) -> Result<(), VaultError> {
        config_entity::Entity::update_many()
            .col_expr(
                ConfigCol::LastSecretKeyRotationAt,
                Expr::value(ts_to_string(&at)),
            )
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn clear_last_snapshot_at(&self) -> Result<(), VaultError> {
        // Null it after `rekey_vault` retires the snapshot store (slice 5.9 ④ / the 5.8
        // self-review bug): re-key drops every snapshot but was leaving the timestamp
        // claiming one exists.
        config_entity::Entity::update_many()
            .col_expr(
                ConfigCol::LastSnapshotAt,
                Expr::value(Option::<String>::None),
            )
            .filter(ConfigCol::Id.eq("default"))
            .exec(self.conn.as_ref())
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )]

    use sea_orm::{EntityTrait, PaginatorTrait};

    use crate::application::vault::ports::VaultRepository;
    use crate::domain::shared::now;
    use crate::domain::vault::entities::VaultConfig;
    use crate::domain::vault::kdf_params::KdfParams;
    use crate::infrastructure::sqlite::vault::VaultDbConnection;
    use crate::infrastructure::sqlite::vault::entities::vault_config::{self as config_entity};

    use super::SqliteVaultRepository;

    /// A pre-4.6 config row: no intrinsic `vault_uuid` yet.
    fn legacy_config() -> VaultConfig {
        VaultConfig {
            id: "default".to_owned(),
            magic: "VEDG".to_owned(),
            schema_version: 1,
            vault_salt: [0u8; 32],
            kdf_params: KdfParams::argon2id_default(),
            verify_hash: [0u8; 32],
            preferred_cipher_suite: 1,
            trash_retention_days: 30,
            audit_retention_days: 90,
            created_at: now(),
            last_unlocked_at: None,
            vault_uuid: None,
            commit_counter: 0,
            backup_dir: None,
            backup_keep_count: None,
            last_snapshot_at: None,
            last_backup_at: None,
            recovery_slot: None,
            last_password_change_at: None,
            last_secret_key_rotation_at: None,
        }
    }

    async fn open_repo() -> (tempfile::TempDir, VaultDbConnection, SqliteVaultRepository) {
        let dir = tempfile::tempdir().unwrap();
        let db = VaultDbConnection::open(&dir.path().join("work.vdb"))
            .await
            .unwrap();
        let repo = SqliteVaultRepository::new(db.handle());
        (dir, db, repo)
    }

    /// The load-bearing test of the additive `vault_uuid` design (slice 4.6a): a
    /// backfill that transitions `vault_uuid` NULL → ULID must **UPDATE** the
    /// singleton row via `save_config`'s upsert-on-`Id`, never INSERT a second one.
    /// (The rejected "make the PK become the uuid" shape would produce a new
    /// conflict key ⇒ a duplicate row ⇒ `load_config`'s unfiltered `.one()`
    /// returning an arbitrary one — silent, unrecoverable config corruption.)
    #[tokio::test]
    async fn vault_uuid_backfill_does_not_duplicate_config_row() {
        let (_dir, db, repo) = open_repo().await;

        let mut config = legacy_config();
        repo.save_config(&config).await.unwrap();

        config.vault_uuid = Some(ulid::Ulid::new().to_string());
        repo.save_config(&config).await.unwrap();

        let count = config_entity::Entity::find()
            .count(db.handle().as_ref())
            .await
            .unwrap();
        assert_eq!(
            count, 1,
            "backfill must UPDATE the singleton row, not INSERT"
        );
        assert_eq!(
            repo.load_config().await.unwrap().vault_uuid,
            config.vault_uuid
        );
    }

    /// The same guarantee through `rewrap_all_deks` (the change-password path),
    /// which carries its OWN upsert + hand-built `ActiveModel`. If `vault_uuid`
    /// were missing from *that* upsert, change-password would drop the identity;
    /// if it changed the conflict key it would duplicate the row.
    #[tokio::test]
    async fn rewrap_config_upsert_keeps_single_row_and_preserves_uuid() {
        let (_dir, db, repo) = open_repo().await;

        let mut config = legacy_config();
        config.vault_uuid = Some(ulid::Ulid::new().to_string());
        repo.save_config(&config).await.unwrap();

        // No entry/tag rewraps — just re-persist the config through the rewrap path.
        repo.rewrap_all_deks(&[], &[], &config).await.unwrap();

        let count = config_entity::Entity::find()
            .count(db.handle().as_ref())
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            repo.load_config().await.unwrap().vault_uuid,
            config.vault_uuid
        );
    }
}
