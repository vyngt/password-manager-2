use std::sync::Arc;

use crate::business::domain::services::VaultService;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;

pub struct UnlockVaultInput {
    pub key: String,
}

pub struct UnlockVaultUseCase {
    vault_service: Arc<dyn VaultService>,
}

impl UnlockVaultUseCase {
    pub fn new(vault_service: Arc<dyn VaultService>) -> Self {
        Self { vault_service }
    }
}

impl BaseUseCase<UnlockVaultInput, bool> for UnlockVaultUseCase {
    async fn execute(&self, input: UnlockVaultInput) -> AppResult<bool> {
        let unlocked = self.vault_service.unlock(&input.key).await;

        Ok(unlocked)
    }
}
