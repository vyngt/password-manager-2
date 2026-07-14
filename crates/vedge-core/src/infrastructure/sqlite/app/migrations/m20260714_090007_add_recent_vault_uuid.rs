use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Slice 5.2.0: carry the vault's intrinsic `vault_uuid` in the recents list so
        // 5.2.2's import can detect a duplicate vault regardless of its on-disk path.
        // Additive + nullable — existing rows keep `NULL` until their next open re-records
        // them with the uuid. (The first additive `alter_table` in the app DB; every prior
        // app migration is a `create_table`.)
        manager
            .alter_table(
                Table::alter()
                    .table(RecentVaults::Table)
                    .add_column_if_not_exists(ColumnDef::new(RecentVaults::VaultUuid).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(RecentVaults::Table)
                    .drop_column(RecentVaults::VaultUuid)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum RecentVaults {
    Table,
    VaultUuid,
}
