use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250913_000002_init_vault"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VaultItem::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VaultItem::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(VaultItem::Title).string().not_null())
                    .col(ColumnDef::new(VaultItem::Kind).string().not_null())
                    .col(ColumnDef::new(VaultItem::Data).string().not_null())
                    .col(
                        ColumnDef::new(VaultItem::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VaultItem::UpdatedAt)
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
            .drop_table(Table::drop().table(VaultItem::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum VaultItem {
    Table,
    Id,
    Title,
    Kind,
    Data,
    CreatedAt,
    UpdatedAt,
}
