use super::entities::ColorScheme;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250914_000002_init_color_scheme"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ColorScheme::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ColorScheme::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ColorScheme::Name).string().not_null())
                    .col(
                        ColumnDef::new(ColorScheme::ColorPrimary)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::ColorSecondary)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::ColorSuccess)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::ColorWarning)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ColorScheme::ColorDanger).string().not_null())
                    .col(
                        ColumnDef::new(ColorScheme::ColorForeground)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::ColorBackground)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ColorScheme::UpdatedAt)
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
            .drop_table(Table::drop().table(ColorScheme::Table).to_owned())
            .await
    }
}
