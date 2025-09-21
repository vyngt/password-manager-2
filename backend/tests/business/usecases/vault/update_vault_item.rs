use backend::business::usecases::{UpdateVaultItemUseCase, UpdateVaultItemInput};
use backend::business::domain::entities::vault_item::{VaultItemData, VaultItemDataCredential, VaultItemKind};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;
use uuid::Uuid;

use crate::business::usecases::vault::test_utils::{create_test_vault_item_with_id, MockVaultItemRepository};

#[tokio::test]
async fn test_update_vault_item_success() {
    let test_id = Uuid::new_v4();
    let existing_item = create_test_vault_item_with_id(test_id);
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![existing_item])
    );
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: test_id.to_string(),
        title: "Updated Title".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.id, test_id);
    assert_eq!(vault_item.title, input.title);
    assert_eq!(vault_item.kind, input.kind);
    match (&vault_item.data, &input.data) {
        (VaultItemData::Credential(cred), VaultItemData::Credential(input_cred)) => {
            assert_eq!(cred.identifier, input_cred.identifier);
            assert_eq!(cred.password, input_cred.password);
            assert_eq!(cred.url, input_cred.url);
        }
    }
}

#[tokio::test]
async fn test_update_vault_item_invalid_uuid() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: "invalid-uuid".to_string(),
        title: "Updated Title".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UuidError(_) => (),
        _ => panic!("Expected UuidError for invalid UUID"),
    }
}

#[tokio::test]
async fn test_update_vault_item_not_found() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: Uuid::new_v4().to_string(),
        title: "Updated Title".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
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
async fn test_update_vault_item_repository_error() {
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: Uuid::new_v4().to_string(),
        title: "Updated Title".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}

#[tokio::test]
async fn test_update_vault_item_with_empty_title() {
    let test_id = Uuid::new_v4();
    let existing_item = create_test_vault_item_with_id(test_id);
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![existing_item])
    );
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: test_id.to_string(),
        title: "".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.title, input.title);
}

#[tokio::test]
async fn test_update_vault_item_preserves_id() {
    let test_id = Uuid::new_v4();
    let existing_item = create_test_vault_item_with_id(test_id);
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![existing_item])
    );
    let use_case = UpdateVaultItemUseCase::new(mock_repo);

    let input = UpdateVaultItemInput {
        id: test_id.to_string(),
        title: "Updated Title".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "updated@example.com".to_string(),
            password: "newpassword123".to_string(),
            url: "https://updated.com".to_string(),
        }),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.id, test_id);
}
