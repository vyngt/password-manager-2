use std::sync::Arc;

use crate::v2::business::domain::entities::vault_record::{VaultRecord, VaultRecordUpdate};
use crate::v2::business::domain::repositories::vault_record::VaultRecordRepository;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;

pub struct UpdateVaultRecordInput {
    id: String,

    name: String,
    url: String,
    login: String,
    key_pass: String,
}

pub struct UpdateVaultRecordUseCase {
    vault_record_repository: Arc<dyn VaultRecordRepository>,
}

impl UpdateVaultRecordUseCase {
    pub fn new(vault_record_repository: Arc<dyn VaultRecordRepository>) -> Self {
        Self {
            vault_record_repository,
        }
    }
}

impl BaseUseCase<UpdateVaultRecordInput, VaultRecord> for UpdateVaultRecordUseCase {
    async fn execute(&self, input: UpdateVaultRecordInput) -> AppResult<VaultRecord> {
        self.vault_record_repository
            .update(
                input.id,
                VaultRecordUpdate {
                    name: input.name,
                    url: input.url,
                    login: input.login,
                    key_pass: input.key_pass,
                },
            )
            .await
    }
}
