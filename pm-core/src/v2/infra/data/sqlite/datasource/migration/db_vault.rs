pub mod m20250913_000001_init_vault;

pub struct Migrator;
use sea_orm_migration::prelude::*;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250913_000001_init_vault::Migration)]
    }
}
