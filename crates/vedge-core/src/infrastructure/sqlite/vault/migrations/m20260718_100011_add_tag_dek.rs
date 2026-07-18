use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Slice 5.6.0 — Tag Keys. Until now `tags` was the ONE thing sealed directly
        // under the vault KEK (no per-row DEK), so a master-password change re-wrapped
        // every entry DEK + `verify_hash` but left tag rows under the OLD KEK — bricking
        // any tagged vault (neither password opens it) and, via ⑬, every snapshot of it.
        //
        // Tags now carry a per-row DEK wrapped under the KEK, exactly like entries. This
        // ADDITIVE nullable column stores that 40-byte wrapped DEK (RFC 3394). A NULL means
        // a LEGACY KEK-sealed tag; it is migrated to a DEK on the next unlock (see
        // `unlock_vault::build_index`) and read through the legacy path until then. Additive
        // + nullable, so an existing vault migrates with zero user action — but the at-rest
        // tag SHAPE changes, so `CURRENT_SCHEMA_VERSION` bumps 1 → 2 (a v1 build must refuse
        // a migrated vault rather than fail mysteriously at tag-decrypt).
        manager
            .alter_table(
                Table::alter()
                    .table(Tags::Table)
                    .add_column_if_not_exists(ColumnDef::new(Tags::DekWrapped).blob().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Tags::Table)
                    .drop_column(Tags::DekWrapped)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Tags {
    Table,
    DekWrapped,
}
