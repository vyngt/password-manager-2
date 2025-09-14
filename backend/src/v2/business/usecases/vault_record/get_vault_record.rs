use std::sync::Arc;

use crate::v2::business::domain::entities::vault_record::VaultRecord;
use crate::v2::business::domain::repositories::vault_record::VaultRecordRepository;
use crate::v2::shared::base::BaseUseCase;

pub struct GetVaultRecordInput {
    id: String,
}

pub struct GetVaultRecordUseCase {
    vault_record_repository: Arc<dyn VaultRecordRepository>,
}

impl GetVaultRecordUseCase {
    pub fn new(vault_record_repository: Arc<dyn VaultRecordRepository>) -> Self {
        Self {
            vault_record_repository,
        }
    }
}

impl BaseUseCase<GetVaultRecordInput, VaultRecord> for GetVaultRecordUseCase {
    async fn execute(&self, input: GetVaultRecordInput) -> anyhow::Result<VaultRecord> {
        self.vault_record_repository.get(input.id).await
    }
}
