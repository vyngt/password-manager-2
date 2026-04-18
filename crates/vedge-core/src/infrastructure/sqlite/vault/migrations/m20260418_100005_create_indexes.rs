use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_entries_trashed")
                    .table(Entries::Table)
                    .col(Entries::IsTrashed)
                    .col(Entries::TrashedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_entries_updated")
                    .table(Entries::Table)
                    .col(Entries::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_entries_accessed")
                    .table(Entries::Table)
                    .col(Entries::AccessedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_audit_entry")
                    .table(AuditLog::Table)
                    .col(AuditLog::EntryId)
                    .col(AuditLog::OccurredAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for name in [
            "idx_audit_entry",
            "idx_entries_accessed",
            "idx_entries_updated",
            "idx_entries_trashed",
        ] {
            manager
                .drop_index(Index::drop().name(name).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Entries {
    Table,
    IsTrashed,
    TrashedAt,
    UpdatedAt,
    AccessedAt,
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    EntryId,
    OccurredAt,
}
