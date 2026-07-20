use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RecentVaults::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecentVaults::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RecentVaults::Path).text().not_null())
                    .col(ColumnDef::new(RecentVaults::DisplayName).text().not_null())
                    .col(ColumnDef::new(RecentVaults::LastOpened).text())
                    .col(
                        ColumnDef::new(RecentVaults::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecentVaults::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RecentVaults {
    Table,
    Id,
    Path,
    DisplayName,
    LastOpened,
    SortOrder,
}
