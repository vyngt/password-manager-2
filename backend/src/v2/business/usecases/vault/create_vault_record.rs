use std::sync::Arc;

use crate::v2::business::domain::entities::vault_record::VaultRecord;
use crate::v2::business::domain::repositories::vault_record::VaultRecordRepository;
use crate::v2::shared::base::BaseUseCase;

pub struct CreateVaultRecordInput {
    pub name: String,
    pub url: String,
    pub login: String,
    pub key_pass: String,
}

pub struct CreateVaultRecordUseCase {
    vault_record_repository: Arc<dyn VaultRecordRepository>,
}

impl CreateVaultRecordUseCase {
    pub fn new(vault_record_repository: Arc<dyn VaultRecordRepository>) -> Self {
        Self {
            vault_record_repository,
        }
    }
}

impl BaseUseCase<CreateVaultRecordInput, VaultRecord> for CreateVaultRecordUseCase {
    async fn execute(&self, input: CreateVaultRecordInput) -> anyhow::Result<VaultRecord> {
        let vault_record =
            VaultRecord::new(None, input.name, input.url, input.login, input.key_pass);

        self.vault_record_repository.create(vault_record).await
    }
}
