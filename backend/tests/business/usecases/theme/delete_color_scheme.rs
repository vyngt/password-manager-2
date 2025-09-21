use backend::business::usecases::{DeleteColorSchemeUseCase, DeleteColorSchemeInput};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::theme::test_utils::{create_test_color_scheme, MockThemeRepository};

#[tokio::test]
async fn test_delete_color_scheme_success() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
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
async fn test_delete_color_scheme_invalid_uuid() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
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
async fn test_delete_color_scheme_not_found() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
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
async fn test_delete_color_scheme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
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
async fn test_delete_color_scheme_returns_correct_id() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
        id: expected_color_scheme.id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.id, expected_color_scheme.id);
}

#[tokio::test]
async fn test_delete_color_scheme_with_different_requested_id() {
    let mock_color_scheme = create_test_color_scheme();
    let requested_id = Uuid::new_v4();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(mock_color_scheme),
    );
    let use_case = DeleteColorSchemeUseCase::new(mock_repo);

    let input = DeleteColorSchemeInput {
        id: requested_id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.id, requested_id); // Should return the requested ID, not the mock's ID
}
