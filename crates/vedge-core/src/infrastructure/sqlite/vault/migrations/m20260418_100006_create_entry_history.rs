use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Encrypted prior-version snapshots. No `dek_wrapped` — snapshots reuse
        // the live entry's stable DEK, so they survive a master-password change.
        manager
            .create_table(
                Table::create()
                    .table(EntryHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EntryHistory::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EntryHistory::EntryId).text().not_null())
                    .col(
                        ColumnDef::new(EntryHistory::Version)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EntryHistory::CipherSuite)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(EntryHistory::Nonce).blob().not_null())
                    .col(ColumnDef::new(EntryHistory::Ciphertext).blob().not_null())
                    .col(ColumnDef::new(EntryHistory::ChangedAt).text().not_null())
                    .to_owned(),
            )
            .await?;

        // List / prune query index: newest-first per entry.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_entry_history_entry")
                    .table(EntryHistory::Table)
                    .col(EntryHistory::EntryId)
                    .col(EntryHistory::Version)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EntryHistory::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EntryHistory {
    Table,
    Id,
    EntryId,
    Version,
    CipherSuite,
    Nonce,
    Ciphertext,
    ChangedAt,
}
