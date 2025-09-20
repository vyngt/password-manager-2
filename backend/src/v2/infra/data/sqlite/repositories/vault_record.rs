use crate::v2::business::domain::repositories::ListVaultRecordsOptions;
use crate::v2::business::domain::{
    entities::vault_record::{VaultRecord, VaultRecordUpdate},
    repositories::vault_record::VaultRecordRepository,
};
use crate::v2::errors::{AppResult, DataOperation};
use crate::v2::infra::data::sqlite::{
    datasource::connection::DataSourceConnection,
    entities::vault::{prelude::*, vault},
    mappers::db_vault::vault::VaultMapper,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::v2::shared::pager::{PaginationMetadata, PaginationOutput};
use sea_orm::QuerySelect;
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
        use crate::v2::infra::data::sqlite::entities::vault::prelude::*;

        let db = self.connection.conn();

        let total_count = Vault::find().count(db).await?;

        let records = Vault::find()
            .limit(input.pagination.limit)
            .offset(input.pagination.offset)
            .all(db)
            .await?
            .iter()
            .map(|v| VaultMapper::to_domain(v.clone()))
            .collect::<Vec<VaultRecord>>();

        Ok(PaginationOutput {
            data: records,
            metadata: PaginationMetadata::calculate_pagination(
                input.pagination.limit,
                input.pagination.offset,
                total_count,
            ),
        })
    }

    async fn get(&self, id: Uuid) -> AppResult<VaultRecord> {
        let db = self.connection.conn();

        let record = Vault::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DataOperation::NotFoundError)?;
        let domain = VaultMapper::to_domain(record);

        Ok(domain)
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
    async fn update(&self, id: Uuid, vault_record: VaultRecordUpdate) -> AppResult<VaultRecord> {
        let db = self.connection.conn();
        let persistence = VaultMapper::to_update(id, vault_record);
        let res = Vault::update(persistence).exec(db).await?;
        let domain = VaultMapper::to_domain(res);

        Ok(domain)
    }
    async fn delete(&self, id: Uuid) -> AppResult<VaultRecord> {
        use sea_orm::Set;

        let db = self.connection.conn();
        let record = Vault::delete(vault::ActiveModel {
            id: Set(id),
            ..Default::default()
        })
        .exec_with_returning(db)
        .await?
        .ok_or(DataOperation::DeleteError)?;

        let domain = VaultMapper::to_domain(record);

        Ok(domain)
    }
}
