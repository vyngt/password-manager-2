use std::sync::Arc;

use crate::v2::business::domain::entities::vault_item::VaultItem;
use crate::v2::business::domain::repositories::{ListVaultItemsOptions, VaultItemRepository};
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;
use crate::v2::shared::pager::{PaginationInput, PaginationOutput};

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
