use async_trait::async_trait;
use vedge_core::business::domain::entities::theme::{
    ColorScheme, Theme, UpdateColorScheme, UpdateTheme,
};
use vedge_core::business::domain::repositories::ThemeRepository;
use vedge_core::errors::AppResult;
use uuid::Uuid;

// Mock implementation of ThemeRepository for testing
pub struct MockThemeRepository {
    pub current_theme: Option<Theme>,
    pub color_scheme: Option<ColorScheme>,
    pub should_fail: bool,
}

impl MockThemeRepository {
    pub fn new() -> Self {
        Self {
            current_theme: None,
            color_scheme: None,
            should_fail: false,
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.current_theme = Some(theme);
        self
    }

    pub fn with_color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    pub fn should_fail(mut self, fail: bool) -> Self {
        self.should_fail = fail;
        self
    }
}

#[async_trait]
impl ThemeRepository for MockThemeRepository {
    async fn list_color_schemes(
        &self,
        input: vedge_core::business::domain::repositories::ListColorSchemesOptions,
    ) -> AppResult<vedge_core::shared::pager::PaginationOutput<ColorScheme>> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        // Create a mock list of color schemes
        let mut color_schemes = vec![
            ColorScheme {
                id: Uuid::new_v4(),
                name: "Default Theme".to_string(),
                color_primary: "#3B82F6".to_string(),
                color_secondary: "#6B7280".to_string(),
                color_success: "#10B981".to_string(),
                color_danger: "#EF4444".to_string(),
                color_warning: "#F59E0B".to_string(),
                color_foreground: "#1F2937".to_string(),
                color_background: "#FFFFFF".to_string(),
            },
            ColorScheme {
                id: Uuid::new_v4(),
                name: "Dark Theme".to_string(),
                color_primary: "#60A5FA".to_string(),
                color_secondary: "#9CA3AF".to_string(),
                color_success: "#34D399".to_string(),
                color_danger: "#F87171".to_string(),
                color_warning: "#FBBF24".to_string(),
                color_foreground: "#F9FAFB".to_string(),
                color_background: "#111827".to_string(),
            },
            ColorScheme {
                id: Uuid::new_v4(),
                name: "Ocean Theme".to_string(),
                color_primary: "#0EA5E9".to_string(),
                color_secondary: "#64748B".to_string(),
                color_success: "#22C55E".to_string(),
                color_danger: "#F97316".to_string(),
                color_warning: "#EAB308".to_string(),
                color_foreground: "#0F172A".to_string(),
                color_background: "#F0F9FF".to_string(),
            },
        ];

        // Apply pagination
        let total_count = color_schemes.len() as u64;
        let start = input.pagination.offset as usize;
        let end = (input.pagination.offset + input.pagination.limit) as usize;

        let paginated_data = if start >= color_schemes.len() {
            vec![]
        } else {
            color_schemes
                .drain(start..end.min(color_schemes.len()))
                .collect()
        };

        let metadata = vedge_core::shared::pager::PaginationMetadata::calculate_pagination(
            input.pagination.limit,
            input.pagination.offset,
            total_count,
        );

        Ok(vedge_core::shared::pager::PaginationOutput {
            data: paginated_data,
            metadata,
        })
    }

    async fn get_color_scheme(&self, _id: Uuid) -> AppResult<ColorScheme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        self.color_scheme.clone().ok_or_else(|| {
            vedge_core::errors::AppError::DataOperationError(
                vedge_core::errors::DataOperation::NotFoundError,
            )
        })
    }

    async fn create_color_scheme(&self, item: ColorScheme) -> AppResult<ColorScheme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }
        Ok(item)
    }

    async fn update_color_scheme(
        &self,
        id: Uuid,
        item: UpdateColorScheme,
    ) -> AppResult<ColorScheme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        if let Some(mut result) = self.color_scheme.clone() {
            result.id = id;
            result.name = item.name;
            result.color_primary = item.color_primary;
            result.color_secondary = item.color_secondary;
            result.color_success = item.color_success;
            result.color_danger = item.color_danger;
            result.color_warning = item.color_warning;
            result.color_foreground = item.color_foreground;
            result.color_background = item.color_background;
            Ok(result)
        } else {
            Err(vedge_core::errors::AppError::DataOperationError(
                vedge_core::errors::DataOperation::NotFoundError,
            ))
        }
    }

    async fn delete_color_scheme(&self, id: Uuid) -> AppResult<ColorScheme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        if let Some(mut result) = self.color_scheme.clone() {
            result.id = id;
            Ok(result)
        } else {
            Err(vedge_core::errors::AppError::DataOperationError(
                vedge_core::errors::DataOperation::NotFoundError,
            ))
        }
    }

    async fn get_current_theme(&self) -> AppResult<Theme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        self.current_theme.clone().ok_or_else(|| {
            vedge_core::errors::AppError::DataOperationError(
                vedge_core::errors::DataOperation::NotFoundError,
            )
        })
    }

    async fn update_theme(&self, id: Uuid, item: UpdateTheme) -> AppResult<Theme> {
        if self.should_fail {
            return Err(vedge_core::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        if let Some(mut theme) = self.current_theme.clone() {
            theme.id = id;
            theme.color_scheme.id = item.color_scheme_id;
            Ok(theme)
        } else {
            Err(vedge_core::errors::AppError::DataOperationError(
                vedge_core::errors::DataOperation::NotFoundError,
            ))
        }
    }
}

pub fn create_test_color_scheme() -> ColorScheme {
    ColorScheme {
        id: Uuid::new_v4(),
        name: "Test Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    }
}

pub fn create_test_theme() -> Theme {
    Theme {
        id: Uuid::new_v4(),
        color_scheme: create_test_color_scheme(),
    }
}
