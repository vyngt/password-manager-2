use std::sync::Arc;

use crate::v2::business::domain::entities::vault_record::VaultRecord;
use crate::v2::business::domain::repositories::{ListVaultRecordsOptions, VaultRecordRepository};
use crate::v2::shared::base::BaseUseCase;
use crate::v2::shared::pager::{PaginationInput, PaginationOutput};

pub struct ListVaultRecordInput {
    pagination: PaginationInput,
}

pub struct ListVaultRecordUseCase {
    vault_record_repository: Arc<dyn VaultRecordRepository>,
}

impl ListVaultRecordUseCase {
    pub fn new(vault_record_repository: Arc<dyn VaultRecordRepository>) -> Self {
        Self {
            vault_record_repository,
        }
    }
}

impl BaseUseCase<ListVaultRecordInput, PaginationOutput<VaultRecord>> for ListVaultRecordUseCase {
    async fn execute(
        &self,
        input: ListVaultRecordInput,
    ) -> anyhow::Result<PaginationOutput<VaultRecord>> {
        self.vault_record_repository
            .list(ListVaultRecordsOptions {
                pagination: input.pagination,
            })
            .await
    }
}
