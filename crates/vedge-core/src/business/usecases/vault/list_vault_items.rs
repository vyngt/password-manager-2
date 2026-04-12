use std::sync::Arc;

use crate::business::domain::entities::vault_item::VaultItem;
use crate::business::domain::repositories::{ListVaultItemsOptions, VaultItemRepository};
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use crate::shared::pager::{PaginationInput, PaginationOutput};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ListVaultItemInput {
    pub pagination: PaginationInput,
}

pub struct ListVaultItemUseCase {
    vault_item_repo: Arc<dyn VaultItemRepository>,
}

impl ListVaultItemUseCase {
    pub fn new(vault_item_repo: Arc<dyn VaultItemRepository>) -> Self {
        Self { vault_item_repo }
    }
}

impl BaseUseCase<ListVaultItemInput, PaginationOutput<VaultItem>> for ListVaultItemUseCase {
    async fn execute(&self, input: ListVaultItemInput) -> AppResult<PaginationOutput<VaultItem>> {
        self.vault_item_repo
            .list(ListVaultItemsOptions {
                pagination: input.pagination,
            })
            .await
    }
}
