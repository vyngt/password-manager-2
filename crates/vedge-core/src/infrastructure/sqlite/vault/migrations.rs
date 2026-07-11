use sea_orm_migration::prelude::*;

mod m20260418_100001_create_vault_config;
mod m20260418_100002_create_tags;
mod m20260418_100003_create_entries;
mod m20260418_100004_create_audit_log;
mod m20260418_100005_create_indexes;
mod m20260418_100006_create_entry_history;
mod m20260711_100007_create_audit_occurred_index;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260418_100001_create_vault_config::Migration),
            Box::new(m20260418_100002_create_tags::Migration),
            Box::new(m20260418_100003_create_entries::Migration),
            Box::new(m20260418_100004_create_audit_log::Migration),
            Box::new(m20260418_100005_create_indexes::Migration),
            Box::new(m20260418_100006_create_entry_history::Migration),
            Box::new(m20260711_100007_create_audit_occurred_index::Migration),
        ]
    }
}
