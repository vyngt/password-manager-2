use std::sync::Arc;

use crate::business::domain::entities::vault_item::{
    UpdateVaultItem, VaultItem, VaultItemData, VaultItemKind,
};
use crate::business::domain::repositories::VaultItemRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateVaultItemInput {
    pub id: String,
    pub title: String,
    pub kind: VaultItemKind,
    pub data: VaultItemData,
}

pub struct UpdateVaultItemUseCase {
    vault_item_repo: Arc<dyn VaultItemRepository>,
}

impl UpdateVaultItemUseCase {
    pub fn new(vault_item_repo: Arc<dyn VaultItemRepository>) -> Self {
        Self { vault_item_repo }
    }
}

impl BaseUseCase<UpdateVaultItemInput, VaultItem> for UpdateVaultItemUseCase {
    async fn execute(&self, input: UpdateVaultItemInput) -> AppResult<VaultItem> {
        let id = Uuid::parse_str(&input.id)?;
        self.vault_item_repo
            .update(
                id,
                UpdateVaultItem {
                    title: input.title,
                    kind: input.kind,
                    data: input.data,
                },
            )
            .await
    }
}
