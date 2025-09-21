use crate::business::domain::entities::theme::ColorScheme;
use crate::business::domain::repositories::ThemeRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DeleteColorSchemeInput {
    pub id: String,
}

pub struct DeleteColorSchemeUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl DeleteColorSchemeUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<DeleteColorSchemeInput, ColorScheme> for DeleteColorSchemeUseCase {
    async fn execute(&self, input: DeleteColorSchemeInput) -> AppResult<ColorScheme> {
        let id = Uuid::parse_str(&input.id)?;
        self.theme_repo.delete_color_scheme(id).await
    }
}
