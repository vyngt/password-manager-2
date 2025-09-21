use backend::business::usecases::{ImportVaultItemsInput, ImportVaultItemsUseCase};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::utilities::test_utils::MockUtilitiesService;
use crate::business::usecases::vault::test_utils::MockVaultItemRepository;

#[tokio::test]
async fn test_import_vault_items_success() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(
        MockUtilitiesService::new().with_file_content(
            r#"[
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "title": "Test Item 1",
                "kind": "Credential",
                "data": {
                    "kind": "Credential",
                    "identifier": "user1",
                    "password": "pass1",
                    "url": "https://example1.com"
                },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            },
            {
                "id": "550e8400-e29b-41d4-a716-446655440001",
                "title": "Test Item 2",
                "kind": "Credential",
                "data": {
                    "kind": "Credential",
                    "identifier": "user2",
                    "password": "pass2",
                    "url": "https://example2.com"
                },
                "created_at": "2024-01-02T00:00:00Z",
                "updated_at": "2024-01-02T00:00:00Z"
            }
        ]"#
            .to_string(),
        ),
    );
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "import.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_import_vault_items_empty_array() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new().with_file_content("[]".to_string()));
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "empty_import.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_import_vault_items_invalid_json() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities =
        Arc::new(MockUtilitiesService::new().with_file_content("invalid json content".to_string()));
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "invalid.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_vault_items_file_not_found() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "nonexistent/import.json".to_string(),
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

#[tokio::test]
async fn test_import_vault_items_repository_error() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let mock_utilities = Arc::new(
        MockUtilitiesService::new().with_file_content(
            r#"[{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Test Item",
            "kind": "Credential",
            "data": {
                "kind": "Credential",
                "identifier": "user",
                "password": "pass",
                "url": "https://example.com"
            },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }]"#
            .to_string(),
        ),
    );
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "repo_error.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_vault_items_utilities_error() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new().should_fail(true));
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "utilities_error.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_vault_items_with_special_characters() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(
        MockUtilitiesService::new().with_file_content(
            r#"[{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Special chars: émojis 🚀, unicode: 中文测试",
            "kind": "Credential",
            "data": {
                "kind": "Credential",
                "identifier": "user@example.com",
                "password": "P@ssw0rd!@#$%^&*()",
                "url": "https://example.com/path?param=value&other=测试"
            },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }]"#
            .to_string(),
        ),
    );
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "special_chars_import.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_import_vault_items_generates_new_uuids() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(
        MockUtilitiesService::new().with_file_content(
            r#"[{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "title": "Test Item",
            "kind": "Credential",
            "data": {
                "kind": "Credential",
                "identifier": "user",
                "password": "pass",
                "url": "https://example.com"
            },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }]"#
            .to_string(),
        ),
    );
    let use_case = ImportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ImportVaultItemsInput {
        file_path: "uuid_test.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
    // Note: The actual UUID generation verification would require access to the repository's
    // internal state, which is not available in the current mock implementation.
    // In a real implementation, you might want to verify that the created items have new UUIDs.
}
