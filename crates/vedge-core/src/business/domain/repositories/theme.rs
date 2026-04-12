use uuid::Uuid;

use crate::shared::pager::{PaginationInput, PaginationOutput};

use super::super::entities::theme::{ColorScheme, Theme, UpdateColorScheme, UpdateTheme};
use crate::errors::AppResult;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ListColorSchemesOptions {
    pub pagination: PaginationInput,
}

#[async_trait::async_trait]
pub trait ThemeRepository: Send + Sync {
    // Color Scheme APIs
    async fn list_color_schemes(
        &self,
        input: ListColorSchemesOptions,
    ) -> AppResult<PaginationOutput<ColorScheme>>;
    async fn get_color_scheme(&self, id: Uuid) -> AppResult<ColorScheme>;
    async fn create_color_scheme(&self, item: ColorScheme) -> AppResult<ColorScheme>;
    async fn update_color_scheme(
        &self,
        id: Uuid,
        item: UpdateColorScheme,
    ) -> AppResult<ColorScheme>;
    async fn delete_color_scheme(&self, id: Uuid) -> AppResult<ColorScheme>;

    // Theme APIs
    async fn get_current_theme(&self) -> AppResult<Theme>;
    async fn update_theme(&self, id: Uuid, item: UpdateTheme) -> AppResult<Theme>;
}
