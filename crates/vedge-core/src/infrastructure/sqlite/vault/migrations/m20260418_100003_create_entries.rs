use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Entries::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Entries::Id).text().not_null().primary_key())
                    .col(
                        ColumnDef::new(Entries::Version)
                            .big_integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Entries::CipherSuite)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(Entries::DekWrapped).blob().not_null())
                    .col(ColumnDef::new(Entries::Nonce).blob().not_null())
                    .col(ColumnDef::new(Entries::Ciphertext).blob().not_null())
                    .col(ColumnDef::new(Entries::CreatedAt).text().not_null())
                    .col(ColumnDef::new(Entries::UpdatedAt).text().not_null())
                    .col(ColumnDef::new(Entries::AccessedAt).text())
                    .col(
                        ColumnDef::new(Entries::IsTrashed)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Entries::TrashedAt).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entries::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Entries {
    Table,
    Id,
    Version,
    CipherSuite,
    DekWrapped,
    Nonce,
    Ciphertext,
    CreatedAt,
    UpdatedAt,
    AccessedAt,
    IsTrashed,
    TrashedAt,
}
