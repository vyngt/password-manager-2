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
                    .name("idx_recent_vaults_last_opened")
                    .table(RecentVaults::Table)
                    .col((RecentVaults::LastOpened, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_known_devices_last_seen")
                    .table(KnownDevices::Table)
                    .col((KnownDevices::LastSeen, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_extension_last_active")
                    .table(ExtensionSessions::Table)
                    .col((ExtensionSessions::LastActiveAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_extension_last_active")
                    .table(ExtensionSessions::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_known_devices_last_seen")
                    .table(KnownDevices::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_recent_vaults_last_opened")
                    .table(RecentVaults::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum RecentVaults {
    Table,
    LastOpened,
}

#[derive(DeriveIden)]
enum KnownDevices {
    Table,
    LastSeen,
}

#[derive(DeriveIden)]
enum ExtensionSessions {
    Table,
    LastActiveAt,
}
