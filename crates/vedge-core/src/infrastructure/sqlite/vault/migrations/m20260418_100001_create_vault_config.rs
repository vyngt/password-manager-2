use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VaultConfig::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VaultConfig::Id)
                            .text()
                            .not_null()
                            .default("default")
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(VaultConfig::Magic)
                            .text()
                            .not_null()
                            .default("VEDG"),
                    )
                    .col(
                        ColumnDef::new(VaultConfig::SchemaVersion)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(ColumnDef::new(VaultConfig::VaultSalt).blob().not_null())
                    .col(ColumnDef::new(VaultConfig::KdfParams).text().not_null())
                    .col(ColumnDef::new(VaultConfig::VerifyHash).blob().not_null())
                    .col(
                        ColumnDef::new(VaultConfig::PreferredCipherSuite)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(VaultConfig::TrashRetentionDays)
                            .integer()
                            .not_null()
                            .default(30),
                    )
                    .col(
                        ColumnDef::new(VaultConfig::AuditRetentionDays)
                            .integer()
                            .not_null()
                            .default(90),
                    )
                    .col(ColumnDef::new(VaultConfig::CreatedAt).text().not_null())
                    .col(ColumnDef::new(VaultConfig::LastUnlockedAt).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VaultConfig::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum VaultConfig {
    Table,
    Id,
    Magic,
    SchemaVersion,
    VaultSalt,
    KdfParams,
    VerifyHash,
    PreferredCipherSuite,
    TrashRetentionDays,
    AuditRetentionDays,
    CreatedAt,
    LastUnlockedAt,
}
