use vedge_core::business::usecases::{UpdateThemeUseCase, UpdateThemeInput};
use vedge_core::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::theme::test_utils::{create_test_theme, MockThemeRepository};

#[tokio::test]
async fn test_update_theme_success() {
    let expected_theme = create_test_theme();
    let mock_repo = Arc::new(MockThemeRepository::new().with_theme(expected_theme.clone()));
    let use_case = UpdateThemeUseCase::new(mock_repo);

    let input = UpdateThemeInput {
        id: expected_theme.id.to_string(),
        color_scheme_id: expected_theme.color_scheme.id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let theme = result.unwrap();
    assert_eq!(theme.id, expected_theme.id);
    assert_eq!(theme.color_scheme.id, expected_theme.color_scheme.id);
}

#[tokio::test]
async fn test_update_theme_invalid_uuid() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = UpdateThemeUseCase::new(mock_repo);

    let input = UpdateThemeInput {
        id: "invalid-uuid".to_string(),
        color_scheme_id: Uuid::new_v4().to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UuidError(_) => (),
        _ => panic!("Expected UuidError for invalid UUID"),
    }
}

#[tokio::test]
async fn test_update_theme_invalid_color_scheme_uuid() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = UpdateThemeUseCase::new(mock_repo);

    let input = UpdateThemeInput {
        id: Uuid::new_v4().to_string(),
        color_scheme_id: "invalid-uuid".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UuidError(_) => (),
        _ => panic!("Expected UuidError for invalid color scheme UUID"),
    }
}

#[tokio::test]
async fn test_update_theme_not_found() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = UpdateThemeUseCase::new(mock_repo);

    let input = UpdateThemeInput {
        id: Uuid::new_v4().to_string(),
        color_scheme_id: Uuid::new_v4().to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::DataOperationError(
            vedge_core::errors::DataOperation::NotFoundError,
        ) => (),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_update_theme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = UpdateThemeUseCase::new(mock_repo);

    let input = UpdateThemeInput {
        id: Uuid::new_v4().to_string(),
        color_scheme_id: Uuid::new_v4().to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}
