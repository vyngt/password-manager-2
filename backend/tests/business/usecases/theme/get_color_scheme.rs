use backend::business::usecases::{GetColorSchemeUseCase, GetColorSchemeInput};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::theme::test_utils::{create_test_color_scheme, MockThemeRepository};

#[tokio::test]
async fn test_get_color_scheme_success() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = GetColorSchemeUseCase::new(mock_repo);

    let input = GetColorSchemeInput {
        id: expected_color_scheme.id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.id, expected_color_scheme.id);
    assert_eq!(color_scheme.name, expected_color_scheme.name);
    assert_eq!(color_scheme.color_primary, expected_color_scheme.color_primary);
}

#[tokio::test]
async fn test_get_color_scheme_invalid_uuid() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = GetColorSchemeUseCase::new(mock_repo);

    let input = GetColorSchemeInput {
        id: "invalid-uuid".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UuidError(_) => (),
        _ => panic!("Expected UuidError for invalid UUID"),
    }
}

#[tokio::test]
async fn test_get_color_scheme_not_found() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = GetColorSchemeUseCase::new(mock_repo);

    let input = GetColorSchemeInput {
        id: Uuid::new_v4().to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::DataOperationError(
            backend::errors::DataOperation::NotFoundError,
        ) => (),
        _ => panic!("Expected NotFound error"),
    }
}

#[tokio::test]
async fn test_get_color_scheme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = GetColorSchemeUseCase::new(mock_repo);

    let input = GetColorSchemeInput {
        id: Uuid::new_v4().to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}

#[tokio::test]
async fn test_get_color_scheme_with_different_ids() {
    let color_scheme1 = create_test_color_scheme();
    let color_scheme2 = create_test_color_scheme();
    let mock_repo = Arc::new(MockThemeRepository::new().with_color_scheme(color_scheme1.clone()));
    let use_case = GetColorSchemeUseCase::new(mock_repo);

    let input = GetColorSchemeInput {
        id: color_scheme2.id.to_string(), // Different ID
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let returned_color_scheme = result.unwrap();
    assert_eq!(returned_color_scheme.id, color_scheme1.id); // Should return the mock's color scheme
    assert_ne!(returned_color_scheme.id, color_scheme2.id);
}
