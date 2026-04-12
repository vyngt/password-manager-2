use crate::config::settings::Settings;
use vedge_core::business::domain::repositories::{ThemeRepository, VaultItemRepository};
use vedge_core::business::domain::services::{UtilitiesService, VaultService};
use vedge_core::infra::data::sqlite::datasource::connection::DataSourceConnection;
use vedge_core::infra::data::sqlite::datasource::migration::{MigratorTrait, ThemeMigrator};
use vedge_core::infra::data::sqlite::repositories::{ThemeRepositoryImpl, VaultItemRepositoryImpl};
use vedge_core::infra::service::utilities::UtilitiesServiceImpl;
use vedge_core::infra::service::vault::VaultServiceImpl;

use std::sync::Arc;

pub struct Registry {
    pub utilities_service: Arc<dyn UtilitiesService>,
    pub vault_service: Arc<dyn VaultService>,
    pub vault_item_repository: Arc<dyn VaultItemRepository>,
    pub theme_repository: Arc<dyn ThemeRepository>,
}

impl Registry {
    pub async fn from_settings(settings: &Settings) -> Self {
        let vault_path = settings
            .get_vault_path()
            .to_string_lossy()
            .replace('\\', "/");
        let theme_path = settings
            .get_theme_path()
            .to_string_lossy()
            .replace('\\', "/");
        let vault_db_source = Arc::new(DataSourceConnection::new(vault_path.as_str()).await);
        let theme_db_source = Arc::new(DataSourceConnection::new(theme_path.as_str()).await);

        // Migrate up theme database
        ThemeMigrator::up(theme_db_source.conn(), None)
            .await
            .unwrap();

        let utilities_service = Arc::new(UtilitiesServiceImpl::new());
        let vault_service = Arc::new(VaultServiceImpl::new(
            vault_db_source.clone(),
            settings.get_vault_key_path(),
        ));

        let vault_item_repository = Arc::new(VaultItemRepositoryImpl::new(vault_db_source.clone()));
        let theme_repository = Arc::new(ThemeRepositoryImpl::new(theme_db_source.clone()));

        Self {
            utilities_service,
            vault_service,
            vault_item_repository,
            theme_repository,
        }
    }
}
