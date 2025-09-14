use std::sync::Arc;

use crate::v2::business::domain::repositories::vault_record::ListInput;
use crate::v2::business::domain::{
    entities::vault_record::{VaultRecord, VaultRecordUpdate},
    repositories::vault_record::VaultRecordRepository,
};
use crate::v2::infra::data::sqlite::{
    datasource::connection::DataSourceConnection,
    entities::vault::{prelude::*, vault},
    mappers::db_vault::vault::VaultMapper,
};

use crate::v2::shared::pager::PaginationOutput;
use sea_orm::entity::prelude::*;
use sea_orm::QueryFilter;

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
    async fn list(&self, input: ListInput) -> anyhow::Result<PaginationOutput<VaultRecord>> {
        todo!()
    }
    async fn get(&self, id: String) -> anyhow::Result<VaultRecord> {
        todo!()
    }
    async fn create(&self, vault_record: VaultRecord) -> anyhow::Result<VaultRecord> {
        use crate::v2::infra::data::sqlite::entities::vault::prelude::*;
        let db = self.connection.conn();

        let persistence = VaultMapper::to_persistence(vault_record);

        let res = Vault::insert(persistence).exec(db).await?;
        let record = Vault::find_by_id(res.last_insert_id)
            .one(db)
            .await?
            .unwrap();

        let domain = VaultMapper::to_domain(record);

        Ok(domain)
    }
    async fn update(
        &self,
        id: String,
        vault_record: VaultRecordUpdate,
    ) -> anyhow::Result<VaultRecord> {
        todo!()
    }
    async fn delete(&self, id: String) -> anyhow::Result<VaultRecord> {
        todo!()
    }
}
