use crate::business::domain::entities::theme::{ColorScheme, UpdateColorScheme};
use crate::business::domain::repositories::{ListColorSchemesOptions, ThemeRepository};
use crate::constants::THEME_ID;
use crate::errors::{AppError, AppResult, DataOperation};
use crate::infra::data::sqlite::{
    datasource::connection::DataSourceConnection,
    entities::theme::{color_scheme, theme},
    mappers::db_theme::{ColorSchemeMapper, ThemeMapper},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::shared::pager::{PaginationMetadata, PaginationOutput};
use sea_orm::QuerySelect;
use sea_orm::entity::prelude::*;

pub struct ThemeRepositoryImpl {
    connection: Arc<DataSourceConnection>,
}

impl ThemeRepositoryImpl {
    pub fn new(connection: Arc<DataSourceConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait::async_trait]
impl ThemeRepository for ThemeRepositoryImpl {
    async fn list_color_schemes(
        &self,
        input: ListColorSchemesOptions,
    ) -> AppResult<PaginationOutput<ColorScheme>> {
        let db = self.connection.conn();

        let total_count = color_scheme::Entity::find().count(db).await?;

        let records = color_scheme::Entity::find()
            .limit(input.pagination.limit)
            .offset(input.pagination.offset)
            .all(db)
            .await?
            .iter()
            .map(|cs| ColorSchemeMapper::to_domain(cs.clone()))
            .collect::<Vec<ColorScheme>>();

        Ok(PaginationOutput {
            data: records,
            metadata: PaginationMetadata::calculate_pagination(
                input.pagination.limit,
                input.pagination.offset,
                total_count,
            ),
        })
    }

    async fn get_color_scheme(&self, id: Uuid) -> AppResult<ColorScheme> {
        let db = self.connection.conn();

        let record = color_scheme::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(DataOperation::NotFoundError)?;
        let domain = ColorSchemeMapper::to_domain(record);

        Ok(domain)
    }

    async fn create_color_scheme(
        &self,
        color_scheme_record: ColorScheme,
    ) -> AppResult<ColorScheme> {
        let db = self.connection.conn();

        let persistence = ColorSchemeMapper::to_persistence(color_scheme_record);
        let res = color_scheme::Entity::insert(persistence).exec(db).await?;
        let record = color_scheme::Entity::find_by_id(res.last_insert_id)
            .one(db)
            .await?
            .ok_or(DataOperation::SaveError)?;

        let domain = ColorSchemeMapper::to_domain(record);

        Ok(domain)
    }

    async fn update_color_scheme(
        &self,
        id: Uuid,
        color_scheme_record: UpdateColorScheme,
    ) -> AppResult<ColorScheme> {
        let db = self.connection.conn();
        let persistence = ColorSchemeMapper::to_update(id, color_scheme_record);
        let res = color_scheme::Entity::update(persistence).exec(db).await?;
        let domain = ColorSchemeMapper::to_domain(res);

        Ok(domain)
    }

    async fn delete_color_scheme(&self, id: Uuid) -> AppResult<ColorScheme> {
        use sea_orm::Set;

        let db = self.connection.conn();

        let record = color_scheme::Entity::delete(color_scheme::ActiveModel {
            id: Set(id),
            ..Default::default()
        })
        .exec_with_returning(db)
        .await?
        .ok_or(DataOperation::DeleteError)?;

        let domain = ColorSchemeMapper::to_domain(record);

        Ok(domain)
    }

    // Theme APIs - placeholder implementations for now
    async fn get_current_theme(
        &self,
    ) -> AppResult<crate::business::domain::entities::theme::Theme> {
        let conn = self.connection.conn();
        let res = theme::Entity::find_by_id(Uuid::parse_str(THEME_ID).unwrap())
            .find_also_related(color_scheme::Entity)
            .one(conn)
            .await?;
        if let Some((theme_record, Some(color_scheme_record))) = res {
            let domain = ThemeMapper::to_domain(theme_record, color_scheme_record);
            Ok(domain)
        } else {
            Err(AppError::DataOperationError(DataOperation::NotFoundError))
        }
    }

    async fn update_theme(
        &self,
        id: Uuid,
        item: crate::business::domain::entities::theme::UpdateTheme,
    ) -> AppResult<crate::business::domain::entities::theme::Theme> {
        let conn = self.connection.conn();
        let persistence = ThemeMapper::to_update(id, item.clone());
        let color_scheme_record = color_scheme::Entity::find_by_id(item.color_scheme_id)
            .one(conn)
            .await?;
        if let Some(color_scheme_record) = color_scheme_record {
            let theme_record = theme::Entity::update(persistence).exec(conn).await?;
            Ok(ThemeMapper::to_domain(theme_record, color_scheme_record))
        } else {
            Err(AppError::DataOperationError(DataOperation::SaveError))
        }
    }
}
