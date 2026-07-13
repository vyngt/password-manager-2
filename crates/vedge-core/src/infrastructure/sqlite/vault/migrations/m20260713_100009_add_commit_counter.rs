use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A monotonic per-vault write counter, bumped on every content mutation and
        // mirrored into the OS keychain (keyed on `vault_uuid`). Unlock compares the
        // file's counter against the keychain baseline: a file that is BEHIND the
        // baseline means the `.vdb` was rolled back to an earlier state (an attacker
        // swapping in an old snapshot, or a restore of an old backup). Slice 5.2c.
        //
        // ADDITIVE, mirroring `vault_uuid` (4.6a): a NOT NULL column with a DEFAULT 0
        // so existing rows adopt a valid baseline on migrate with zero user action.
        // The column is written ONLY by the repo-layer bump and restore's re-baseline;
        // it is deliberately omitted from the two config-upsert `update_columns` lists
        // so an unlock's `save_config` cannot clobber the live counter with a stale read.
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(VaultConfig::CommitCounter)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .drop_column(VaultConfig::CommitCounter)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    CommitCounter,
}
