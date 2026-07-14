use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Snapshot + backup bookkeeping on `vault_config` (slice 5.2.1):
        //   - `backup_dir`         — an optional override of the default `<home>/snapshots`
        //                            store location.
        //   - `backup_keep_count`  — opt-in snapshot retention (Decision ⑯); UNSET ⇒ keep
        //                            everything (the default).
        //   - `last_snapshot_at`   — when a snapshot was last taken.
        //   - `last_backup_at`     — when a `.vbk` was last written. 🔴 SEPARATE from
        //                            `last_snapshot_at` (Decision ⑧): a snapshot is NOT a
        //                            backup, and the health card must read only THIS column.
        //
        // All four are ADDITIVE nullable columns (like `vault_uuid`), so `schema_version`
        // stays 1 and existing vaults migrate with zero user action. `last_snapshot_at` /
        // `last_backup_at` are written by targeted repo updates and deliberately OMITTED from
        // the config-upsert `update_columns` lists, so an unlock's stale `save_config` cannot
        // clobber them — the `commit_counter` (5.2c) precedent.
        for col in [
            VaultConfig::BackupDir,
            VaultConfig::LastSnapshotAt,
            VaultConfig::LastBackupAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(VaultConfig::Table)
                        .add_column_if_not_exists(ColumnDef::new(col).text().null())
                        .to_owned(),
                )
                .await?;
        }
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(VaultConfig::BackupKeepCount)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in [
            VaultConfig::BackupDir,
            VaultConfig::BackupKeepCount,
            VaultConfig::LastSnapshotAt,
            VaultConfig::LastBackupAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(VaultConfig::Table)
                        .drop_column(col)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    BackupDir,
    BackupKeepCount,
    LastSnapshotAt,
    LastBackupAt,
}
