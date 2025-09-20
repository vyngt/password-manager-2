use std::sync::Arc;

use uuid::Uuid;

use crate::v2::business::domain::entities::vault_item::VaultItem;
use crate::v2::business::domain::repositories::vault_item::VaultItemRepository;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DeleteVaultItemInput {
    pub id: String,
}

pub struct DeleteVaultItemUseCase {
    vault_item_repo: Arc<dyn VaultItemRepository>,
}

impl DeleteVaultItemUseCase {
    pub fn new(vault_item_repo: Arc<dyn VaultItemRepository>) -> Self {
        Self { vault_item_repo }
    }
}

impl BaseUseCase<DeleteVaultItemInput, VaultItem> for DeleteVaultItemUseCase {
    async fn execute(&self, input: DeleteVaultItemInput) -> AppResult<VaultItem> {
        let id = Uuid::parse_str(&input.id)?;
        self.vault_item_repo.delete(id).await
    }
}
