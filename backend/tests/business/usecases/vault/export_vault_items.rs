use backend::business::usecases::{ExportVaultItemsInput, ExportVaultItemsUseCase};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::utilities::test_utils::MockUtilitiesService;
use crate::business::usecases::vault::test_utils::MockVaultItemRepository;

#[tokio::test]
async fn test_export_vault_items_success() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "export.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_vault_items_with_empty_vault() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "empty_export.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_vault_items_repository_error() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "error_export.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_export_vault_items_utilities_error() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new().should_fail(true));
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "utilities_error.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_err());
}

#[tokio::test]
async fn test_export_vault_items_with_nested_path() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "exports/backup/vault_items.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_vault_items_with_special_characters_in_path() {
    // Arrange
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let mock_utilities = Arc::new(MockUtilitiesService::new());
    let use_case = ExportVaultItemsUseCase::new(mock_repo, mock_utilities);

    let input = ExportVaultItemsInput {
        file_path: "exports/backup-2024/vault_items_export.json".to_string(),
    };

    // Act
    let result = use_case.execute(input).await;

    // Assert
    assert!(result.is_ok());
}
