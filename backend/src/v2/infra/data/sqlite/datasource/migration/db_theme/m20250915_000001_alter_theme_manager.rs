use super::entities::ThemeManager;
use sea_orm::Statement;
use sea_orm_migration::prelude::*;
use uuid::uuid;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250915_000001_alter_theme_manager"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let alter_stmt = Statement::from_sql_and_values(
            manager.get_database_backend(),
            r#"ALTER TABLE theme_manager ADD color_scheme_id uuid_text REFERENCES color_scheme(id) ON DELETE RESTRICT;"#,
            [],
        );
        db.execute(alter_stmt).await?;

        let stmt = Statement::from_sql_and_values(
            manager.get_database_backend(),
            r#"UPDATE `theme_manager` SET `color_scheme_id` = ? WHERE `id` = ?"#,
            [
                uuid!("14368642-5c48-4aa6-8a5c-fe7346e4cd1e").into(),
                1.into(),
            ],
        );
        db.execute(stmt).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ThemeManager::Table)
                    .drop_column(ThemeManager::ColorSchemeId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
