use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KnownDevices::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(KnownDevices::DeviceId)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(KnownDevices::DisplayName).text().not_null())
                    .col(ColumnDef::new(KnownDevices::PublicKey).blob().not_null())
                    .col(ColumnDef::new(KnownDevices::FirstSeen).text().not_null())
                    .col(ColumnDef::new(KnownDevices::LastSeen).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KnownDevices::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum KnownDevices {
    Table,
    DeviceId,
    DisplayName,
    PublicKey,
    FirstSeen,
    LastSeen,
}
