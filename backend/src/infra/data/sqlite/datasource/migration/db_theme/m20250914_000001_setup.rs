use super::entities::Theme;
use crate::constants::THEME_ID;
use sea_orm::Statement;
use sea_orm_migration::prelude::*;
use uuid::uuid;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250914_000001_setup"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager
            .create_table(
                Table::create()
                    .table(Theme::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Theme::Id).uuid().not_null().primary_key())
                    .to_owned(),
            )
            .await;

        let db = manager.get_connection();

        let stmt = Statement::from_sql_and_values(
            manager.get_database_backend(),
            r#"INSERT INTO `theme` (`id`) VALUES (?)"#,
            [uuid!(THEME_ID).into()],
        );
        db.execute(stmt).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Theme::Table).to_owned())
            .await
    }
}
