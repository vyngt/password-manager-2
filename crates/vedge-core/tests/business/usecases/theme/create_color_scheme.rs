use vedge_core::business::usecases::{CreateColorSchemeUseCase, CreateColorSchemeInput};
use vedge_core::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::theme::test_utils::MockThemeRepository;

#[tokio::test]
async fn test_create_color_scheme_success() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = CreateColorSchemeUseCase::new(mock_repo);
    let input = CreateColorSchemeInput {
        name: "Test Color Scheme".to_string(),
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
async fn test_create_color_scheme_generates_unique_id() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = CreateColorSchemeUseCase::new(mock_repo);
    let input = CreateColorSchemeInput {
        name: "Test Color Scheme".to_string(),
        color_primary: "#3B82F6".to_string(),
        color_secondary: "#6B7280".to_string(),
        color_success: "#10B981".to_string(),
        color_danger: "#EF4444".to_string(),
        color_warning: "#F59E0B".to_string(),
        color_foreground: "#1F2937".to_string(),
        color_background: "#FFFFFF".to_string(),
    };

    let result1 = use_case.execute(input.clone()).await;
    let result2 = use_case.execute(input).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let color_scheme1 = result1.unwrap();
    let color_scheme2 = result2.unwrap();

    assert_ne!(color_scheme1.id, color_scheme2.id);
}

#[tokio::test]
async fn test_create_color_scheme_with_empty_name() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = CreateColorSchemeUseCase::new(mock_repo);
    let input = CreateColorSchemeInput {
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
async fn test_create_color_scheme_with_special_characters() {
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = CreateColorSchemeUseCase::new(mock_repo);
    let input = CreateColorSchemeInput {
        name: "Special Theme 🎨".to_string(),
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
async fn test_create_color_scheme_repository_error() {
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = CreateColorSchemeUseCase::new(mock_repo);
    let input = CreateColorSchemeInput {
        name: "Test Color Scheme".to_string(),
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
