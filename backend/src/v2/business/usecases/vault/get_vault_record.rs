use crate::v2::business::domain::entities::vault_record::VaultRecord;
use crate::v2::business::domain::repositories::vault_record::VaultRecordRepository;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

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
    async fn execute(&self, input: GetVaultRecordInput) -> AppResult<VaultRecord> {
        let id = Uuid::parse_str(&input.id)?;
        self.vault_record_repository.get(id).await
    }
}
