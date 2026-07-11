use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The audit page sorts + range-filters by `occurred_at` alone. The only
        // existing audit index — `idx_audit_entry (entry_id, occurred_at)` — has
        // the wrong leading column and cannot serve it, so an unfiltered
        // newest-first read full-scans + sorts. Idempotent + additive: old vaults
        // gain it on next open via the migrator.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audit_occurred")
                    .table(AuditLog::Table)
                    .col(AuditLog::OccurredAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_audit_occurred").to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    OccurredAt,
}
