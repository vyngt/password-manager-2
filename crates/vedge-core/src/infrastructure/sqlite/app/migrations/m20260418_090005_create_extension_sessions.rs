use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExtensionSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExtensionSessions::SessionId)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ExtensionSessions::Browser).text().not_null())
                    .col(ColumnDef::new(ExtensionSessions::ProfileName).text())
                    .col(
                        ColumnDef::new(ExtensionSessions::SessionKey)
                            .blob()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExtensionSessions::CreatedAt)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ExtensionSessions::LastActiveAt).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExtensionSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ExtensionSessions {
    Table,
    SessionId,
    Browser,
    ProfileName,
    SessionKey,
    CreatedAt,
    LastActiveAt,
}
