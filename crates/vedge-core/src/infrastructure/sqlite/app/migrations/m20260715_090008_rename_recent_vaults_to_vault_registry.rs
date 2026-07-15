use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

/// Slice 5.2.4: `recent_vaults` was never a "recently opened" list — it is the authoritative
/// registry mapping a vault's `vault_uuid` to a home path a human can name, and (5.2.4) the
/// delete tombstone. Rename the table to `vault_registry` to match its intent.
///
/// Data-preserving: `ALTER TABLE ... RENAME TO` keeps every row and column; the `last_opened`
/// index is dropped and recreated under the new name (`SQLite` keeps an index attached across a
/// table rename but under its old name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_recent_vaults_last_opened")
            .await?;
        db.execute_unprepared("ALTER TABLE recent_vaults RENAME TO vault_registry")
            .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_vault_registry_last_opened \
             ON vault_registry (last_opened DESC)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_vault_registry_last_opened")
            .await?;
        db.execute_unprepared("ALTER TABLE vault_registry RENAME TO recent_vaults")
            .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_recent_vaults_last_opened \
             ON recent_vaults (last_opened DESC)",
        )
        .await?;
        Ok(())
    }
}
