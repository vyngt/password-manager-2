use crate::v2::business::domain::services::VaultService;
use crate::v2::infra::data::sqlite::DataSourceConnection;
use async_trait::async_trait;
use std::sync::Arc;

pub struct VaultServiceImpl {
    encryption_datasource: Arc<DataSourceConnection>,
}

impl VaultServiceImpl {
    pub fn new(encryption_datasource: Arc<DataSourceConnection>) -> Self {
        Self {
            encryption_datasource,
        }
    }

    async fn check_if_unlocked(&self) -> bool {
        let stmt = "SELECT id FROM pre_setup where id=1;";
        match self.encryption_datasource.execute_raw(&stmt).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn apply_migrations(&self) -> bool {
        use crate::v2::infra::data::sqlite::datasource::migration::VaultMigrator;
        use sea_orm_migration::MigratorTrait;

        match VaultMigrator::up(self.encryption_datasource.conn(), None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl VaultService for VaultServiceImpl {
    async fn unlock(&self, key: &str) -> bool {
        let stmt = format!("PRAGMA key={key};");
        let result = self.encryption_datasource.execute_raw(&stmt).await;
        match result {
            Ok(_) => {
                if self.apply_migrations().await && self.check_if_unlocked().await {
                    return true;
                }

                false
            }
            Err(_) => false,
        }
    }

    async fn change_key(&self, new_key: &str) -> bool {
        let stmt = format!("PRAGMA rekey={new_key};");
        let result = self.encryption_datasource.execute_raw(&stmt).await;
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
