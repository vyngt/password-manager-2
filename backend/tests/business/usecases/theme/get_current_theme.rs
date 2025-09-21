use backend::business::usecases::GetCurrentThemeUseCase;
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::theme::test_utils::{MockThemeRepository, create_test_theme};

#[tokio::test]
async fn test_get_current_theme_success() {
    let expected_theme = create_test_theme();
    let mock_repo = Arc::new(MockThemeRepository::new().with_theme(expected_theme.clone()));
    let use_case = GetCurrentThemeUseCase::new(mock_repo);

    let result = use_case.execute(()).await;

    assert!(result.is_ok());
    let theme = result.unwrap();
    assert_eq!(theme.id, expected_theme.id);
    assert_eq!(theme.color_scheme.id, expected_theme.color_scheme.id);
}

#[tokio::test]
async fn test_get_current_theme_not_found() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = GetCurrentThemeUseCase::new(mock_repo);

    let result = use_case.execute(()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::DataOperationError(
            backend::errors::DataOperation::NotFoundError,
        ) => (),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_get_current_theme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = GetCurrentThemeUseCase::new(mock_repo);

    let result = use_case.execute(()).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}
