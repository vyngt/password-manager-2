use std::sync::Arc;

use crate::v2::business::domain::entities::vault_item::{
    VaultItem, VaultItemData, VaultItemKind, VaultItemUpdate,
};
use crate::v2::business::domain::repositories::vault_item::VaultItemRepository;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;
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
                VaultItemUpdate {
                    title: input.title,
                    kind: input.kind,
                    data: input.data,
                },
            )
            .await
    }
}
