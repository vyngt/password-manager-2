pub mod entities;
mod m20250914_000001_setup;
mod m20250914_000002_init_color_scheme;
mod m20250914_000003_seeding_color_scheme;
mod m20250915_000001_alter_theme_manager;

pub struct Migrator;
use sea_orm_migration::prelude::*;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250914_000001_setup::Migration),
            Box::new(m20250914_000002_init_color_scheme::Migration),
            Box::new(m20250914_000003_seeding_color_scheme::Migration),
            Box::new(m20250915_000001_alter_theme_manager::Migration),
        ]
    }
}
