use std::path::PathBuf;
use std::sync::Arc;

use crate::business::domain::entities::vault_item::VaultItem;
use crate::business::domain::repositories::VaultItemRepository;
use crate::business::domain::services::UtilitiesService;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ImportVaultItemsInput {
    pub file_path: String,
}

pub struct ImportVaultItemsUseCase {
    vault_item_repository: Arc<dyn VaultItemRepository>,
    utilities_service: Arc<dyn UtilitiesService>,
}

impl ImportVaultItemsUseCase {
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

impl BaseUseCase<ImportVaultItemsInput, ()> for ImportVaultItemsUseCase {
    async fn execute(&self, input: ImportVaultItemsInput) -> AppResult<()> {
        // Read file content
        let path = PathBuf::from(input.file_path);
        let json_content = self.utilities_service.read_from_file(path).await?;

        // Parse JSON to get vault items
        let mut vault_items: Vec<VaultItem> = serde_json::from_str(&json_content).map_err(|e| {
            crate::errors::AppError::UnknownError(format!("JSON deserialization failed: {}", e))
        })?;

        // Generate new UUIDs for all items
        for item in &mut vault_items {
            item.id = uuid::Uuid::new_v4();
        }

        // Create all vault items
        self.vault_item_repository.create_many(vault_items).await?;

        Ok(())
    }
}
