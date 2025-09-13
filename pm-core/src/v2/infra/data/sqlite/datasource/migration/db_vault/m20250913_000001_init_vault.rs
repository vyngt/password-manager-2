use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250913_000001_init_vault"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Vault::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Vault::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Vault::Name).string().not_null())
                    .col(ColumnDef::new(Vault::Url).string().not_null())
                    .col(ColumnDef::new(Vault::Login).string().not_null())
                    .col(ColumnDef::new(Vault::KeyPass).string().not_null())
                    .col(
                        ColumnDef::new(Vault::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Vault::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Vault::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Vault {
    Table,
    Id,
    Name,
    Url,
    Login,
    KeyPass,
    CreatedAt,
    UpdatedAt,
}
