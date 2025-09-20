use std::sync::Arc;

use crate::v2::business::domain::repositories::ListVaultRecordsOptions;
use crate::v2::business::domain::{
    entities::vault_record::{VaultRecord, VaultRecordUpdate},
    repositories::vault_record::VaultRecordRepository,
};
use crate::v2::errors::{AppError, AppResult, DataOperation};
use crate::v2::infra::data::sqlite::{
    datasource::connection::DataSourceConnection,
    entities::vault::{prelude::*, vault},
    mappers::db_vault::vault::VaultMapper,
};

use crate::v2::shared::pager::PaginationOutput;
use sea_orm::QueryFilter;
use sea_orm::entity::prelude::*;

pub struct VaultRecordRepositoryImpl {
    connection: Arc<DataSourceConnection>,
}

impl VaultRecordRepositoryImpl {
    pub fn new(connection: Arc<DataSourceConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait::async_trait]
impl VaultRecordRepository for VaultRecordRepositoryImpl {
    async fn list(
        &self,
        input: ListVaultRecordsOptions,
    ) -> AppResult<PaginationOutput<VaultRecord>> {
        todo!()
    }
    async fn get(&self, id: String) -> AppResult<VaultRecord> {
        todo!()
    }

    async fn create(&self, vault_record: VaultRecord) -> AppResult<VaultRecord> {
        use crate::v2::infra::data::sqlite::entities::vault::prelude::*;
        let db = self.connection.conn();

        let persistence = VaultMapper::to_persistence(vault_record);

        let res = Vault::insert(persistence).exec(db).await?;
        let record = Vault::find_by_id(res.last_insert_id)
            .one(db)
            .await?
            .ok_or(DataOperation::SaveError)?;

        let domain = VaultMapper::to_domain(record);

        Ok(domain)
    }
    async fn update(&self, id: String, vault_record: VaultRecordUpdate) -> AppResult<VaultRecord> {
        todo!()
    }
    async fn delete(&self, id: String) -> AppResult<VaultRecord> {
        todo!()
    }
}
