use vedge_core::business::usecases::{UpdateColorSchemeUseCase, UpdateColorSchemeInput};
use vedge_core::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::theme::test_utils::{create_test_color_scheme, MockThemeRepository};

#[tokio::test]
async fn test_update_color_scheme_success() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: expected_color_scheme.id.to_string(),
        name: "Updated Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.name, input.name);
    assert_eq!(color_scheme.color_primary, input.color_primary);
    assert_eq!(color_scheme.color_secondary, input.color_secondary);
    assert_eq!(color_scheme.color_success, input.color_success);
    assert_eq!(color_scheme.color_danger, input.color_danger);
    assert_eq!(color_scheme.color_warning, input.color_warning);
    assert_eq!(color_scheme.color_foreground, input.color_foreground);
    assert_eq!(color_scheme.color_background, input.color_background);
}

#[tokio::test]
async fn test_update_color_scheme_invalid_uuid() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: "invalid-uuid".to_string(),
        name: "Updated Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UuidError(_) => (),
        _ => panic!("Expected UuidError for invalid UUID"),
    }
}

#[tokio::test]
async fn test_update_color_scheme_not_found() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: Uuid::new_v4().to_string(),
        name: "Updated Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
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
async fn test_update_color_scheme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: Uuid::new_v4().to_string(),
        name: "Updated Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}

#[tokio::test]
async fn test_update_color_scheme_with_empty_name() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: expected_color_scheme.id.to_string(),
        name: "".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.name, input.name);
}

#[tokio::test]
async fn test_update_color_scheme_preserves_id() {
    let expected_color_scheme = create_test_color_scheme();
    let mock_repo = Arc::new(
        MockThemeRepository::new().with_color_scheme(expected_color_scheme.clone()),
    );
    let use_case = UpdateColorSchemeUseCase::new(mock_repo);
    let input = UpdateColorSchemeInput {
        id: expected_color_scheme.id.to_string(),
        name: "Updated Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };
    let expected_id = Uuid::parse_str(&input.id).unwrap();

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let color_scheme = result.unwrap();
    assert_eq!(color_scheme.id, expected_id);
}
