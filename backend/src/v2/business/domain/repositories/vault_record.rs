use crate::v2::shared::pager::{PaginationInput, PaginationOutput};

use super::super::entities::vault_record::{VaultRecord, VaultRecordUpdate};
use crate::v2::errors::AppResult;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ListVaultRecordsOptions {
    pub pagination: PaginationInput,
}

#[async_trait::async_trait]
pub trait VaultRecordRepository: Send + Sync {
    async fn list(
        &self,
        input: ListVaultRecordsOptions,
    ) -> AppResult<PaginationOutput<VaultRecord>>;
    async fn get(&self, id: uuid::Uuid) -> AppResult<VaultRecord>;
    async fn create(&self, vault_record: VaultRecord) -> AppResult<VaultRecord>;
    async fn update(
        &self,
        id: uuid::Uuid,
        vault_record: VaultRecordUpdate,
    ) -> AppResult<VaultRecord>;
    async fn delete(&self, id: uuid::Uuid) -> AppResult<VaultRecord>;
}
