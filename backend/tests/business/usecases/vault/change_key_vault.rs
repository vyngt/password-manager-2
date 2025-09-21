use backend::business::usecases::{ChangeKeyVaultUseCase, ChangeKeyVaultInput};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::vault::test_utils::MockVaultService;

#[tokio::test]
async fn test_change_key_vault_success() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(true));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "new_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(changed);
}

#[tokio::test]
async fn test_change_key_vault_failure() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(false));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "weak_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(!changed);
}

#[tokio::test]
async fn test_change_key_vault_with_empty_key() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(false));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(!changed);
}

#[tokio::test]
async fn test_change_key_vault_with_special_characters() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(true));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "n3w_p@ssw0rd!@#$%^&*()".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(changed);
}

#[tokio::test]
async fn test_change_key_vault_with_unicode_characters() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(true));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "🔑new_password123".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(changed);
}

#[tokio::test]
async fn test_change_key_vault_with_very_long_key() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(true));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let long_key = "b".repeat(1000);
    let input = ChangeKeyVaultInput {
        key: long_key,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(changed);
}

#[tokio::test]
async fn test_change_key_vault_service_failure() {
    let mock_service = Arc::new(MockVaultService::new().should_fail(true));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "any_new_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(!changed); // Service failure returns false
}

#[tokio::test]
async fn test_change_key_vault_with_same_as_old_key() {
    let mock_service = Arc::new(MockVaultService::new().with_change_key_result(false));
    let use_case = ChangeKeyVaultUseCase::new(mock_service);

    let input = ChangeKeyVaultInput {
        key: "same_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let changed = result.unwrap();
    assert!(!changed); // Should fail if same as old key
}
