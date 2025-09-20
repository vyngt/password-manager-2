use std::sync::Arc;

use crate::v2::business::domain::services::VaultService;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;

pub struct ChangeKeyVaultInput {
    key: String,
}

pub struct ChangeKeyVaultUseCase {
    vault_service: Arc<dyn VaultService>,
}

impl ChangeKeyVaultUseCase {
    pub fn new(vault_service: Arc<dyn VaultService>) -> Self {
        Self { vault_service }
    }
}

impl BaseUseCase<ChangeKeyVaultInput, bool> for ChangeKeyVaultUseCase {
    async fn execute(&self, input: ChangeKeyVaultInput) -> AppResult<bool> {
        let unlocked = self.vault_service.change_key(&input.key).await;

        Ok(unlocked)
    }
}
