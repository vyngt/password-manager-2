use sea_orm_migration::prelude::*;

mod m20260418_090001_create_recent_vaults;
mod m20260418_090002_create_app_settings;
mod m20260418_090003_create_themes;
mod m20260418_090004_create_known_devices;
mod m20260418_090005_create_extension_sessions;
mod m20260418_090006_create_indexes;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260418_090001_create_recent_vaults::Migration),
            Box::new(m20260418_090002_create_app_settings::Migration),
            Box::new(m20260418_090003_create_themes::Migration),
            Box::new(m20260418_090004_create_known_devices::Migration),
            Box::new(m20260418_090005_create_extension_sessions::Migration),
            Box::new(m20260418_090006_create_indexes::Migration),
        ]
    }
}
