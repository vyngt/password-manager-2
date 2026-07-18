use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Slice 5.7 — Recovery Key. An opt-in second credential (a 32-byte random
        // Recovery Key, `RK1-…`) reconstructs the vault KEK when the master password
        // is forgotten. This ADDITIVE nullable BLOB stores the 40-byte slot
        // `AES-KW(wrap_key = KEK_rec, payload = KEK)` (RFC 3394). NULL ⇒ recovery is
        // off; a vault ignores the column unless enrolled.
        //
        // 🟢 NO `schema_version` bump (unlike 5.6.0's tag_dek): this column changes no
        // EXISTING at-rest shape — an older build opening an enrolled vault simply
        // ignores the slot (offers no recovery), and a NULL slot needs no migration.
        // The `vault_uuid` (4.6a) / `commit_counter` (5.2c) precedent. (A pre-5.7 build
        // that CHANGES the password of an enrolled vault leaves the slot stale → dead
        // recovery on downgrade; documented as a known limitation, not a data-loss.)
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(VaultConfig::RecoverySlot).blob().null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(VaultConfig::Table)
                    .drop_column(VaultConfig::RecoverySlot)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    RecoverySlot,
}
