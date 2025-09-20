use crate::business::domain::entities::vault_item::VaultItem;
use crate::business::domain::repositories::vault_item::VaultItemRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetVaultItemInput {
    pub id: String,
}

pub struct GetVaultItemUseCase {
    vault_item_repo: Arc<dyn VaultItemRepository>,
}

impl GetVaultItemUseCase {
    pub fn new(vault_item_repo: Arc<dyn VaultItemRepository>) -> Self {
        Self { vault_item_repo }
    }
}

impl BaseUseCase<GetVaultItemInput, VaultItem> for GetVaultItemUseCase {
    async fn execute(&self, input: GetVaultItemInput) -> AppResult<VaultItem> {
        let id = Uuid::parse_str(&input.id)?;
        self.vault_item_repo.get(id).await
    }
}
