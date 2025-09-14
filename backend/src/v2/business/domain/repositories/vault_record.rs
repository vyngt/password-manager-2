use crate::v2::shared::pager::{PaginationInput, PaginationOutput};

use super::super::entities::vault_record::{VaultRecord, VaultRecordUpdate};
use anyhow;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ListInput {
    pub pagination: PaginationInput,
}

#[async_trait::async_trait]
pub trait VaultRecordRepository: Send + Sync {
    async fn list(&self, input: ListInput) -> anyhow::Result<PaginationOutput<VaultRecord>>;
    async fn get(&self, id: String) -> anyhow::Result<VaultRecord>;
    async fn create(&self, vault_record: VaultRecord) -> anyhow::Result<VaultRecord>;
    async fn update(
        &self,
        id: String,
        vault_record: VaultRecordUpdate,
    ) -> anyhow::Result<VaultRecord>;
    async fn delete(&self, id: String) -> anyhow::Result<VaultRecord>;
}
