use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Credential-age bookkeeping on `vault_config` (slice 5.9 ③):
        //   - `last_password_change_at`     — when the master password's KEK was last refreshed.
        //   - `last_secret_key_rotation_at` — when the Secret Key was last rotated.
        //
        // Both are ADDITIVE nullable columns (like `last_snapshot_at` / `recovery_slot`), so
        // `schema_version` stays 2 and existing vaults migrate with zero user action — a `NULL`
        // reads as "unknown", which the credential-age nudge treats as `created_at`, never
        // "never". Written by the credential paths in the rewrap transaction (via
        // `rewrap_all_deks`'s `update_columns`) and by targeted repo updates, and deliberately
        // OMITTED from the config-upsert `update_columns`, so a stale `save_config` at unlock
        // cannot clobber them — the `last_snapshot_at` / `recovery_slot` precedent.
        for col in [
            VaultConfig::LastPasswordChangeAt,
            VaultConfig::LastSecretKeyRotationAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(VaultConfig::Table)
                        .add_column_if_not_exists(ColumnDef::new(col).text().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in [
            VaultConfig::LastPasswordChangeAt,
            VaultConfig::LastSecretKeyRotationAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(VaultConfig::Table)
                        .drop_column(col)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    LastPasswordChangeAt,
    LastSecretKeyRotationAt,
}
