use backend::business::usecases::{ReadFromFileInput, ReadFromFileUseCase};
use backend::shared::base::BaseUseCase;
use std::path::PathBuf;
use std::sync::Arc;

use crate::business::usecases::utilities::test_utils::MockUtilitiesService;

#[tokio::test]
async fn test_read_from_file_success() {
    // Arrange
    let expected_content = "Hello, World!";
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("test.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_with_empty_content() {
    // Arrange
    let expected_content = "";
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("empty.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_with_large_content() {
    // Arrange
    let expected_content = "A".repeat(10000);
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.clone()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("large.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_with_special_characters() {
    // Arrange
    let expected_content = "Special chars: émojis 🚀, unicode: 中文测试, symbols: @#$%^&*()";
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("special.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_with_nested_path() {
    // Arrange
    let expected_content = "Content in nested folder";
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("nested/folder/structure/file.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_service_error() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new().should_fail(true));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("error.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::FileReadError(msg) => {
            assert_eq!(msg, "Mock read error");
        }
        _ => panic!("Expected FileReadError for service failure"),
    }
}

#[tokio::test]
async fn test_read_from_file_with_json_content() {
    // Arrange
    let expected_content = r#"{
        "name": "Test User",
        "email": "test@example.com",
        "settings": {
            "theme": "dark",
            "notifications": true
        }
    }"#;
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("config.json"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_with_default_content() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("default.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, "Mock file content");
}

#[tokio::test]
async fn test_read_from_file_with_multiline_content() {
    // Arrange
    let expected_content = "Line 1\nLine 2\nLine 3\nWith special chars: émojis 🚀\nEnd";
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_file_content(expected_content.to_string()));
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("multiline.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content, expected_content);
}

#[tokio::test]
async fn test_read_from_file_not_found() {
    // Arrange
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = ReadFromFileUseCase::new(mock_service);

    let input = ReadFromFileInput {
        path: PathBuf::from("nonexistent/file.txt"),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::FileNotFoundError => {
            // Expected error
        }
        _ => panic!("Expected FileNotFoundError for missing file"),
    }
}
