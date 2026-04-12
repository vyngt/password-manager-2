use crate::shared::pager::{PaginationInput, PaginationOutput};

use super::super::entities::vault_item::{UpdateVaultItem, VaultItem};
use crate::errors::AppResult;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ListVaultItemsOptions {
    pub pagination: PaginationInput,
}

#[async_trait::async_trait]
pub trait VaultItemRepository: Send + Sync {
    async fn all(&self) -> AppResult<Vec<VaultItem>>;
    async fn list(&self, input: ListVaultItemsOptions) -> AppResult<PaginationOutput<VaultItem>>;
    async fn get(&self, id: uuid::Uuid) -> AppResult<VaultItem>;
    async fn create(&self, item: VaultItem) -> AppResult<VaultItem>;
    async fn create_many(&self, items: Vec<VaultItem>) -> AppResult<usize>;
    async fn update(&self, id: uuid::Uuid, item: UpdateVaultItem) -> AppResult<VaultItem>;
    async fn delete(&self, id: uuid::Uuid) -> AppResult<VaultItem>;
}
