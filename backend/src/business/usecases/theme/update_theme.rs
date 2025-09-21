use crate::business::domain::entities::theme::{Theme, UpdateTheme};
use crate::business::domain::repositories::ThemeRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateThemeInput {
    pub id: String,
    pub color_scheme_id: String,
}

pub struct UpdateThemeUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl UpdateThemeUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<UpdateThemeInput, Theme> for UpdateThemeUseCase {
    async fn execute(&self, input: UpdateThemeInput) -> AppResult<Theme> {
        let id = Uuid::parse_str(&input.id)?;
        let color_scheme_id = Uuid::parse_str(&input.color_scheme_id)?;

        let update_theme = UpdateTheme { color_scheme_id };

        self.theme_repo.update_theme(id, update_theme).await
    }
}
