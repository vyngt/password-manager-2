use crate::business::domain::entities::theme::Theme;
use crate::business::domain::repositories::ThemeRepository;
use crate::errors::AppResult;
use crate::shared::base::BaseUseCase;
use std::sync::Arc;

pub struct GetCurrentThemeUseCase {
    theme_repo: Arc<dyn ThemeRepository>,
}

impl GetCurrentThemeUseCase {
    pub fn new(theme_repo: Arc<dyn ThemeRepository>) -> Self {
        Self { theme_repo }
    }
}

impl BaseUseCase<(), Theme> for GetCurrentThemeUseCase {
    async fn execute(&self, _input: ()) -> AppResult<Theme> {
        self.theme_repo.get_current_theme().await
    }
}
