use sea_orm::Statement;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250913_000001_setup"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager
            .create_table(
                Table::create()
                    .table(PreSetup::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PreSetup::Id)
                            .integer()
                            .auto_increment()
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await;

        let db = manager.get_connection();

        let stmt = Statement::from_sql_and_values(
            manager.get_database_backend(),
            r#"INSERT INTO `pre_setup` (`id`) VALUES (?)"#,
            [1.into()],
        );
        db.execute(stmt).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PreSetup::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PreSetup {
    Table,
    Id,
}
