use crate::v2::infra::data::sqlite::entities::theme::color_scheme;
use chrono::Utc;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::entity::*;
use uuid::uuid;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250914_000003_seeding_color_scheme"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let vortex = color_scheme::ActiveModel {
            id: Set(uuid!("14368642-5c48-4aa6-8a5c-fe7346e4cd1e")),
            name: Set("Vortex".to_string()),
            color_primary: Set("#F41FC6".to_string()),
            color_secondary: Set("#03A9F4".to_string()),
            color_success: Set("#1EC887".to_string()),
            color_danger: Set("#F42563".to_string()),
            color_warning: Set("#E1C124".to_string()),
            color_foreground: Set("#FFFFFF".to_string()),
            color_background: Set("#000000".to_string()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        color_scheme::Entity::insert_many(vec![vortex])
            .exec(db)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        color_scheme::Entity::delete_by_id(uuid!("14368642-5c48-4aa6-8a5c-fe7346e4cd1e"))
            .exec(db)
            .await?;

        Ok(())
    }
}
