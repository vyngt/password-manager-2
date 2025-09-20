use crate::config::settings::Settings;
use backend::business::domain::repositories::VaultItemRepository;
use backend::business::domain::services::{UtilitiesService, VaultService};
use backend::infra::data::sqlite::datasource::connection::DataSourceConnection;
use backend::infra::data::sqlite::repositories::VaultItemRepositoryImpl;
use backend::infra::service::utilities::UtilitiesServiceImpl;
use backend::infra::service::vault::VaultServiceImpl;

use std::sync::Arc;

pub struct Registry {
    pub utilities_service: Arc<dyn UtilitiesService>,
    pub vault_service: Arc<dyn VaultService>,
    pub vault_item_repository: Arc<dyn VaultItemRepository>,
}

impl Registry {
    pub async fn from_settings(settings: &Settings) -> Self {
        let vault_path = settings
            .get_vault_path()
            .to_string_lossy()
            .replace('\\', "/");
        let source = Arc::new(DataSourceConnection::new(vault_path.as_str()).await);

        let utilities_service = Arc::new(UtilitiesServiceImpl::new());
        let vault_service = Arc::new(VaultServiceImpl::new(
            source.clone(),
            settings.get_vault_key_path(),
        ));

        let vault_item_repository = Arc::new(VaultItemRepositoryImpl::new(source.clone()));

        Self {
            utilities_service,
            vault_service,
            vault_item_repository,
        }
    }
}
