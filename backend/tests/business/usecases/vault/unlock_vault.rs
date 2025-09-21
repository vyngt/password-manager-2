use backend::business::usecases::{UnlockVaultUseCase, UnlockVaultInput};
use backend::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::vault::test_utils::MockVaultService;

#[tokio::test]
async fn test_unlock_vault_success() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(true));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "correct_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(unlocked);
}

#[tokio::test]
async fn test_unlock_vault_failure() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(false));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "wrong_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(!unlocked);
}

#[tokio::test]
async fn test_unlock_vault_with_empty_key() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(false));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(!unlocked);
}

#[tokio::test]
async fn test_unlock_vault_with_special_characters() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(true));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "p@ssw0rd!@#$%^&*()".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(unlocked);
}

#[tokio::test]
async fn test_unlock_vault_with_unicode_characters() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(true));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "🔐password123".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(unlocked);
}

#[tokio::test]
async fn test_unlock_vault_with_very_long_key() {
    let mock_service = Arc::new(MockVaultService::new().with_unlock_result(true));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let long_key = "a".repeat(1000);
    let input = UnlockVaultInput {
        key: long_key,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(unlocked);
}

#[tokio::test]
async fn test_unlock_vault_service_failure() {
    let mock_service = Arc::new(MockVaultService::new().should_fail(true));
    let use_case = UnlockVaultUseCase::new(mock_service);

    let input = UnlockVaultInput {
        key: "any_password".to_string(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let unlocked = result.unwrap();
    assert!(!unlocked); // Service failure returns false
}
