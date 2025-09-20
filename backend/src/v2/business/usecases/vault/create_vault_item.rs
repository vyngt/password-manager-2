use std::sync::Arc;

use crate::v2::business::domain::entities::vault_item::{VaultItem, VaultItemData, VaultItemKind};
use crate::v2::business::domain::repositories::vault_item::VaultItemRepository;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateVaultItemInput {
    pub title: String,
    pub kind: VaultItemKind,
    pub data: VaultItemData,
}

pub struct CreateVaultItemUseCase {
    vault_item_repo: Arc<dyn VaultItemRepository>,
}

impl CreateVaultItemUseCase {
    pub fn new(vault_item_repo: Arc<dyn VaultItemRepository>) -> Self {
        Self { vault_item_repo }
    }
}

impl BaseUseCase<CreateVaultItemInput, VaultItem> for CreateVaultItemUseCase {
    async fn execute(&self, input: CreateVaultItemInput) -> AppResult<VaultItem> {
        let vault_record = VaultItem::new(None, input.title, input.kind, input.data);

        self.vault_item_repo.create(vault_record).await
    }
}
