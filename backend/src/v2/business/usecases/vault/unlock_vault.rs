use std::sync::Arc;

use crate::v2::business::domain::services::VaultService;
use crate::v2::errors::AppResult;
use crate::v2::shared::base::BaseUseCase;

pub struct UnlockVaultInput {
    key: String,
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
