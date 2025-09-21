use crate::business::usecases::theme::test_utils::MockThemeRepository;
use backend::business::usecases::theme::{ListColorSchemesInput, ListColorSchemesUseCase};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

#[tokio::test]
async fn test_list_color_schemes_success() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 10,
        offset: 0,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let pagination_output = result.unwrap();

    // Should return 3 mock color schemes
    assert_eq!(pagination_output.data.len(), 3);

    // Check metadata
    assert_eq!(pagination_output.metadata.total_count, 3);
    assert_eq!(pagination_output.metadata.items_per_page, 10);
    assert_eq!(pagination_output.metadata.current_page, 1);
    assert_eq!(pagination_output.metadata.total_pages, 1);
    assert!(!pagination_output.metadata.has_next_page);
    assert!(!pagination_output.metadata.has_previous_page);

    // Check that we have the expected color scheme names
    let names: Vec<&String> = pagination_output.data.iter().map(|cs| &cs.name).collect();
    assert!(names.contains(&&"Default Theme".to_string()));
    assert!(names.contains(&&"Dark Theme".to_string()));
    assert!(names.contains(&&"Ocean Theme".to_string()));
}

#[tokio::test]
async fn test_list_color_schemes_with_pagination() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 2,
        offset: 0,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let pagination_output = result.unwrap();

    // Should return 2 color schemes (limit = 2)
    assert_eq!(pagination_output.data.len(), 2);

    // Check metadata
    assert_eq!(pagination_output.metadata.total_count, 3);
    assert_eq!(pagination_output.metadata.items_per_page, 2);
    assert_eq!(pagination_output.metadata.current_page, 1);
    assert_eq!(pagination_output.metadata.total_pages, 2);
    assert!(pagination_output.metadata.has_next_page);
    assert!(!pagination_output.metadata.has_previous_page);
}

#[tokio::test]
async fn test_list_color_schemes_second_page() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 2,
        offset: 2,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let pagination_output = result.unwrap();

    // Should return 1 color scheme (remaining items)
    assert_eq!(pagination_output.data.len(), 1);

    // Check metadata
    assert_eq!(pagination_output.metadata.total_count, 3);
    assert_eq!(pagination_output.metadata.items_per_page, 2);
    assert_eq!(pagination_output.metadata.current_page, 2);
    assert_eq!(pagination_output.metadata.total_pages, 2);
    assert!(!pagination_output.metadata.has_next_page);
    assert!(pagination_output.metadata.has_previous_page);
}

#[tokio::test]
async fn test_list_color_schemes_empty_result() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 10,
        offset: 100, // Offset beyond available data
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let pagination_output = result.unwrap();

    // Should return empty data
    assert_eq!(pagination_output.data.len(), 0);

    // Check metadata
    assert_eq!(pagination_output.metadata.total_count, 3);
    assert_eq!(pagination_output.metadata.items_per_page, 10);
    assert_eq!(pagination_output.metadata.current_page, 11); // 100/10 + 1
    assert_eq!(pagination_output.metadata.total_pages, 1);
    assert!(!pagination_output.metadata.has_next_page);
    assert!(pagination_output.metadata.has_previous_page);
}

#[tokio::test]
async fn test_list_color_schemes_repository_error() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new().should_fail(true));
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 10,
        offset: 0,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error, backend::errors::AppError::UnknownError(_)));
}

#[tokio::test]
async fn test_list_color_schemes_zero_limit() {
    // Arrange
    let mock_repo = Arc::new(MockThemeRepository::new());
    let use_case = ListColorSchemesUseCase::new(mock_repo);
    let input = ListColorSchemesInput {
        limit: 0,
        offset: 0,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let pagination_output = result.unwrap();

    // Should return empty data with zero limit
    assert_eq!(pagination_output.data.len(), 0);

    // Check metadata
    assert_eq!(pagination_output.metadata.total_count, 3);
    assert_eq!(pagination_output.metadata.items_per_page, 0);
    assert_eq!(pagination_output.metadata.current_page, 1);
    assert_eq!(pagination_output.metadata.total_pages, 1); // When limit is 0, total_pages is 1
    assert!(!pagination_output.metadata.has_next_page);
    assert!(!pagination_output.metadata.has_previous_page);
}
