use crate::business::domain::repositories::ListVaultItemsOptions;
use crate::business::domain::{
    entities::vault_item::{VaultItem as VaultItemDomain, VaultItemUpdate},
    repositories::vault_item::VaultItemRepository,
};
use crate::errors::{AppResult, DataOperation};
use crate::infra::data::sqlite::{
    datasource::connection::DataSourceConnection,
    entities::vault::{prelude::*, vault_item},
    mappers::db_vault::vault_item::VaultMapper,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::shared::pager::{PaginationMetadata, PaginationOutput};
use sea_orm::QuerySelect;
use sea_orm::entity::prelude::*;

pub struct VaultItemRepositoryImpl {
    connection: Arc<DataSourceConnection>,
}

impl VaultItemRepositoryImpl {
    pub fn new(connection: Arc<DataSourceConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait::async_trait]
impl VaultItemRepository for VaultItemRepositoryImpl {
    async fn list(
        &self,
        input: ListVaultItemsOptions,
    ) -> AppResult<PaginationOutput<VaultItemDomain>> {
        use crate::infra::data::sqlite::entities::vault::prelude::*;

        let db = self.connection.conn();

        let total_count = VaultItem::find().count(db).await?;

        let records = VaultItem::find()
            .limit(input.pagination.limit)
            .offset(input.pagination.offset)
            .all(db)
            .await?
            .iter()
            .map(|v| VaultMapper::to_domain(v.clone()))
            .collect::<Vec<VaultItemDomain>>();

        Ok(PaginationOutput {
            data: records,
            metadata: PaginationMetadata::calculate_pagination(
                input.pagination.limit,
                input.pagination.offset,
                total_count,
            ),
        })
    }

    async fn get(&self, id: Uuid) -> AppResult<VaultItemDomain> {
        let db = self.connection.conn();

        let record = VaultItem::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DataOperation::NotFoundError)?;
        let domain = VaultMapper::to_domain(record);

        Ok(domain)
    }

    async fn create(&self, vault_record: VaultItemDomain) -> AppResult<VaultItemDomain> {
        use crate::infra::data::sqlite::entities::vault::prelude::*;
        let db = self.connection.conn();

        let persistence = VaultMapper::to_persistence(vault_record);
        let res = VaultItem::insert(persistence).exec(db).await?;
        let record = VaultItem::find_by_id(res.last_insert_id)
            .one(db)
            .await?
            .ok_or(DataOperation::SaveError)?;

        let domain = VaultMapper::to_domain(record);

        Ok(domain)
    }
    async fn update(&self, id: Uuid, vault_record: VaultItemUpdate) -> AppResult<VaultItemDomain> {
        let db = self.connection.conn();
        let persistence = VaultMapper::to_update(id, vault_record);
        let res = VaultItem::update(persistence).exec(db).await?;
        let domain = VaultMapper::to_domain(res);

        Ok(domain)
    }
    async fn delete(&self, id: Uuid) -> AppResult<VaultItemDomain> {
        use sea_orm::Set;

        let db = self.connection.conn();

        let record = VaultItem::delete(vault_item::ActiveModel {
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
