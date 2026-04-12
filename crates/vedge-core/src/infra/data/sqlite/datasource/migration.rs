pub mod db_theme;
pub mod db_vault;
pub use sea_orm_migration::MigratorTrait;

pub use db_theme::Migrator as ThemeMigrator;
pub use db_vault::Migrator as VaultMigrator;
