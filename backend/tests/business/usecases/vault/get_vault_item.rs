use backend::business::usecases::{GetVaultItemUseCase, GetVaultItemInput};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::vault::test_utils::{create_test_vault_item_with_id, MockVaultItemRepository};

#[tokio::test]
async fn test_get_vault_item_success() {
    let test_id = Uuid::new_v4();
    let expected_item = create_test_vault_item_with_id(test_id);
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![expected_item.clone()])
    );
    let use_case = GetVaultItemUseCase::new(mock_repo);

    let input = GetVaultItemInput {
        id: test_id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.id, expected_item.id);
    assert_eq!(vault_item.title, expected_item.title);
    assert_eq!(vault_item.kind, expected_item.kind);
}

#[tokio::test]
async fn test_get_vault_item_invalid_uuid() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = GetVaultItemUseCase::new(mock_repo);

    let input = GetVaultItemInput {
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
async fn test_get_vault_item_not_found() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = GetVaultItemUseCase::new(mock_repo);

    let input = GetVaultItemInput {
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
async fn test_get_vault_item_repository_error() {
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let use_case = GetVaultItemUseCase::new(mock_repo);

    let input = GetVaultItemInput {
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
async fn test_get_vault_item_with_different_ids() {
    let item1_id = Uuid::new_v4();
    let item2_id = Uuid::new_v4();
    let item1 = create_test_vault_item_with_id(item1_id);
    let item2 = create_test_vault_item_with_id(item2_id);
    
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![item1.clone(), item2.clone()])
    );
    let use_case = GetVaultItemUseCase::new(mock_repo);

    let input = GetVaultItemInput {
        id: item1_id.to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let returned_item = result.unwrap();
    assert_eq!(returned_item.id, item1.id);
    assert_ne!(returned_item.id, item2.id);
}
