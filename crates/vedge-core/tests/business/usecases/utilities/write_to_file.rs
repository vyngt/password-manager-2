use vedge_core::business::usecases::{WriteToFileInput, WriteToFileUseCase};
use vedge_core::shared::base::BaseUseCase;
use std::path::PathBuf;
use std::sync::Arc;

use crate::business::usecases::utilities::test_utils::MockUtilitiesService;

#[tokio::test]
async fn test_write_to_file_success() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let input = WriteToFileInput {
        path: PathBuf::from("test.txt"),
        content: "Hello, World!".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_with_empty_content() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let input = WriteToFileInput {
        path: PathBuf::from("empty.txt"),
        content: "".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_with_large_content() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let large_content = "A".repeat(10000);
    let input = WriteToFileInput {
        path: PathBuf::from("large.txt"),
        content: large_content,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_with_special_characters() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let special_content =
        "Special chars: émojis 🚀, unicode: 中文测试, symbols: @#$%^&*()".to_string();
    let input = WriteToFileInput {
        path: PathBuf::from("special.txt"),
        content: special_content,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_with_nested_path() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let input = WriteToFileInput {
        path: PathBuf::from("nested/folder/structure/file.txt"),
        content: "Content in nested folder".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_service_error() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new().should_fail(true));
    let use_case = WriteToFileUseCase::new(mock_service);

    let input = WriteToFileInput {
        path: PathBuf::from("error.txt"),
        content: "This should fail".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::FileWriteError(msg) => {
            assert_eq!(msg, "Mock write error");
        }
        _ => panic!("Expected FileWriteError for service failure"),
    }
}

#[tokio::test]
async fn test_write_to_file_with_json_content() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let json_content = r#"{
        "name": "Test User",
        "email": "test@example.com",
        "settings": {
            "theme": "dark",
            "notifications": true
        }
    }"#
    .to_string();

    let input = WriteToFileInput {
        path: PathBuf::from("config.json"),
        content: json_content,
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_to_file_directory_not_found() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = WriteToFileUseCase::new(mock_service);

    let input = WriteToFileInput {
        path: PathBuf::from("nonexistent/directory/file.txt"),
        content: "This should fail due to missing directory".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::DirectoryNotFoundError => {
            // Expected error
        }
        _ => panic!("Expected DirectoryNotFoundError for missing directory"),
    }
}
