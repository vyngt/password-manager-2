use crate::business::domain::entities::theme::ColorScheme;
use crate::business::domain::repositories::ThemeRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CreateColorSchemeInput {
    pub name: String,
    pub color_primary: String,
    pub color_secondary: String,
    pub color_success: String,
    pub color_danger: String,
    pub color_warning: String,
    pub color_foreground: String,
    pub color_background: String,
}

pub struct CreateColorSchemeUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl CreateColorSchemeUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<CreateColorSchemeInput, ColorScheme> for CreateColorSchemeUseCase {
    async fn execute(&self, input: CreateColorSchemeInput) -> AppResult<ColorScheme> {
        let color_scheme = ColorScheme {
            id: Uuid::new_v4(),
            name: input.name,
            color_primary: input.color_primary,
            color_secondary: input.color_secondary,
            color_success: input.color_success,
            color_danger: input.color_danger,
            color_warning: input.color_warning,
            color_foreground: input.color_foreground,
            color_background: input.color_background,
        };

        self.theme_repo.create_color_scheme(color_scheme).await
    }
}
