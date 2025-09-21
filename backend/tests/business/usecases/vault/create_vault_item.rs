use backend::business::usecases::{CreateVaultItemUseCase, CreateVaultItemInput};
use backend::business::domain::entities::vault_item::{VaultItemData, VaultItemDataCredential, VaultItemKind};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::vault::test_utils::MockVaultItemRepository;

#[tokio::test]
async fn test_create_vault_item_success() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = CreateVaultItemUseCase::new(mock_repo);

    let input = CreateVaultItemInput {
        title: "Test Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.title, input.title);
    assert_eq!(vault_item.kind, input.kind);
    match vault_item.data {
        VaultItemData::Credential(cred) => {
            assert_eq!(cred.identifier, "test@example.com");
            assert_eq!(cred.password, "password123");
            assert_eq!(cred.url, "https://example.com");
        }
    }
}

#[tokio::test]
async fn test_create_vault_item_generates_unique_id() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = CreateVaultItemUseCase::new(mock_repo);

    let input = CreateVaultItemInput {
        title: "Test Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    };

    let result1 = use_case.execute(input.clone()).await;
    let result2 = use_case.execute(input).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let vault_item1 = result1.unwrap();
    let vault_item2 = result2.unwrap();

    assert_ne!(vault_item1.id, vault_item2.id);
}

#[tokio::test]
async fn test_create_vault_item_with_empty_title() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = CreateVaultItemUseCase::new(mock_repo);

    let input = CreateVaultItemInput {
        title: "".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.title, input.title);
}

#[tokio::test]
async fn test_create_vault_item_with_special_characters() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = CreateVaultItemUseCase::new(mock_repo);

    let input = CreateVaultItemInput {
        title: "Special Item 🎯".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "special@example.com".to_string(),
            password: "p@ssw0rd!".to_string(),
            url: "https://special-site.com".to_string(),
        }),
    };

    let result = use_case.execute(input.clone()).await;

    assert!(result.is_ok());
    let vault_item = result.unwrap();
    assert_eq!(vault_item.title, input.title);
}

#[tokio::test]
async fn test_create_vault_item_repository_error() {
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let use_case = CreateVaultItemUseCase::new(mock_repo);

    let input = CreateVaultItemInput {
        title: "Test Credential".to_string(),
        kind: VaultItemKind::Credential,
        data: VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        backend::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}
