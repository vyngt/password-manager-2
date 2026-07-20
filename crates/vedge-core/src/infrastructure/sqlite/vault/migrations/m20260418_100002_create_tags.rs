use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tags::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tags::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(Tags::Nonce).blob().not_null())
                    .col(ColumnDef::new(Tags::Ciphertext).blob().not_null())
                    .col(ColumnDef::new(Tags::CreatedAt).text().not_null())
                    .col(ColumnDef::new(Tags::UpdatedAt).text().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tags::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    Id,
    Nonce,
    Ciphertext,
    CreatedAt,
    UpdatedAt,
}
