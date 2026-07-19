use sea_orm_migration::prelude::*;

mod m20260418_100001_create_vault_config;
mod m20260418_100002_create_tags;
mod m20260418_100003_create_entries;
mod m20260418_100004_create_audit_log;
mod m20260418_100005_create_indexes;
mod m20260418_100006_create_entry_history;
mod m20260711_100007_create_audit_occurred_index;
mod m20260711_100008_add_vault_uuid;
mod m20260713_100009_add_commit_counter;
mod m20260714_100010_add_snapshot_backup_fields;
mod m20260718_100011_add_tag_dek;
mod m20260718_100012_add_recovery_slot;
mod m20260719_100013_add_credential_age_fields;

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
            Box::new(m20260711_100008_add_vault_uuid::Migration),
            Box::new(m20260713_100009_add_commit_counter::Migration),
            Box::new(m20260714_100010_add_snapshot_backup_fields::Migration),
            Box::new(m20260718_100011_add_tag_dek::Migration),
            Box::new(m20260718_100012_add_recovery_slot::Migration),
            Box::new(m20260719_100013_add_credential_age_fields::Migration),
        ]
    }
}
