use crate::business::domain::entities::theme::ColorScheme;
use crate::business::domain::repositories::{ListColorSchemesOptions, ThemeRepository};
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use crate::shared::pager::{PaginationInput, PaginationOutput};
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ListColorSchemesInput {
    pub limit: u64,
    pub offset: u64,
}

pub struct ListColorSchemesUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl ListColorSchemesUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<ListColorSchemesInput, PaginationOutput<ColorScheme>> for ListColorSchemesUseCase {
    async fn execute(
        &self,
        input: ListColorSchemesInput,
    ) -> AppResult<PaginationOutput<ColorScheme>> {
        let options = ListColorSchemesOptions {
            pagination: PaginationInput {
                limit: input.limit,
                offset: input.offset,
            },
        };

        self.theme_repo.list_color_schemes(options).await
    }
}
