use std::path::PathBuf;
use std::sync::Arc;

use crate::business::domain::repositories::VaultItemRepository;
use crate::business::domain::services::UtilitiesService;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExportVaultItemsInput {
    pub file_path: String,
}

pub struct ExportVaultItemsUseCase {
    vault_item_repository: Arc<dyn VaultItemRepository>,
    utilities_service: Arc<dyn UtilitiesService>,
}

impl ExportVaultItemsUseCase {
    pub fn new(
        vault_item_repository: Arc<dyn VaultItemRepository>,
        utilities_service: Arc<dyn UtilitiesService>,
    ) -> Self {
        Self {
            vault_item_repository,
            utilities_service,
        }
    }
}

impl BaseUseCase<ExportVaultItemsInput, ()> for ExportVaultItemsUseCase {
    async fn execute(&self, input: ExportVaultItemsInput) -> AppResult<()> {
        // Get all vault items
        let vault_items = self.vault_item_repository.all().await?;

        // Convert to JSON
        let json_content = serde_json::to_string_pretty(&vault_items).map_err(|e| {
            crate::errors::AppError::UnknownError(format!("JSON serialization failed: {}", e))
        })?;

        // Write to file
        let path = PathBuf::from(input.file_path);
        self.utilities_service
            .write_to_file(path, json_content)
            .await?;

        Ok(())
    }
}
