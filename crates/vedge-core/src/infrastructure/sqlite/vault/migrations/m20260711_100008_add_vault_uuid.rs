use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // A vault's only identity used to be its file path (`VaultId(PathBuf)`);
        // moving the `.vdb` made it "a different vault", hard-blocking LAN Sync and
        // anonymizing every backup. Add an intrinsic ULID.
        //
        // ADDITIVE, not a PK swap: `save_config`/`rewrap_all_deks` upsert on `Id`,
        // so changing the primary key would INSERT a second row and `load_config`'s
        // unfiltered `.one()` would then return an arbitrary one — silent config
        // corruption. A new nullable column is minted on create and backfilled on
        // first open, so pre-4.6 vaults gain it with zero user action.
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .add_column_if_not_exists(ColumnDef::new(VaultConfig::VaultUuid).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .drop_column(VaultConfig::VaultUuid)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    VaultUuid,
}
