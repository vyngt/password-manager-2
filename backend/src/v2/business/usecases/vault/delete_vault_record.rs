use std::sync::Arc;

use crate::v2::business::domain::entities::vault_record::VaultRecord;
use crate::v2::business::domain::repositories::vault_record::VaultRecordRepository;
use crate::v2::shared::base::BaseUseCase;

pub struct DeleteVaultRecordInput {
    id: String,
}

pub struct DeleteVaultRecordUseCase {
    vault_record_repository: Arc<dyn VaultRecordRepository>,
}

impl DeleteVaultRecordUseCase {
    pub fn new(vault_record_repository: Arc<dyn VaultRecordRepository>) -> Self {
        Self {
            vault_record_repository,
        }
    }
}

impl BaseUseCase<DeleteVaultRecordInput, VaultRecord> for DeleteVaultRecordUseCase {
    async fn execute(&self, input: DeleteVaultRecordInput) -> anyhow::Result<VaultRecord> {
        self.vault_record_repository.delete(input.id).await
    }
}
