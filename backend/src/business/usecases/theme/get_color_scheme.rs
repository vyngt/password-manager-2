use crate::business::domain::entities::theme::ColorScheme;
use crate::business::domain::repositories::ThemeRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GetColorSchemeInput {
    pub id: String,
}

pub struct GetColorSchemeUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl GetColorSchemeUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<GetColorSchemeInput, ColorScheme> for GetColorSchemeUseCase {
    async fn execute(&self, input: GetColorSchemeInput) -> AppResult<ColorScheme> {
        let id = Uuid::parse_str(&input.id)?;
        self.theme_repo.get_color_scheme(id).await
    }
}
