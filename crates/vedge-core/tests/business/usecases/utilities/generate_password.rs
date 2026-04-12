use vedge_core::business::usecases::{GeneratePasswordInput, GeneratePasswordUseCase};
use vedge_core::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::utilities::test_utils::MockUtilitiesService;

#[tokio::test]
async fn test_generate_password_success() {
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_password("TestPassword123!".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 16,
        upper: true,
        lower: true,
        digits: true,
        special: true,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "TestPassword123!");
}

#[tokio::test]
async fn test_generate_password_with_default_mock() {
    let mock_service = Arc::new(MockUtilitiesService::new());
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 12,
        upper: true,
        lower: false,
        digits: true,
        special: false,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "MockPassword123!");
}

#[tokio::test]
async fn test_generate_password_with_minimal_options() {
    let mock_service = Arc::new(MockUtilitiesService::new().with_password("abc".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 3,
        upper: false,
        lower: true,
        digits: false,
        special: false,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "abc");
}

#[tokio::test]
async fn test_generate_password_with_maximum_options() {
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_password("ComplexP@ssw0rd!".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 50,
        upper: true,
        lower: true,
        digits: true,
        special: true,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "ComplexP@ssw0rd!");
}

#[tokio::test]
async fn test_generate_password_with_zero_length() {
    let mock_service = Arc::new(MockUtilitiesService::new().with_password("".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 0,
        upper: false,
        lower: false,
        digits: false,
        special: false,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "");
}

#[tokio::test]
async fn test_generate_password_with_negative_length() {
    let mock_service = Arc::new(MockUtilitiesService::new().with_password("".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: -5,
        upper: true,
        lower: true,
        digits: true,
        special: true,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "");
}

#[tokio::test]
async fn test_generate_password_with_only_uppercase() {
    let mock_service = Arc::new(MockUtilitiesService::new().with_password("ABCDEFGH".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 8,
        upper: true,
        lower: false,
        digits: false,
        special: false,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "ABCDEFGH");
}

#[tokio::test]
async fn test_generate_password_with_only_digits() {
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_password("1234567890".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 10,
        upper: false,
        lower: false,
        digits: true,
        special: false,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "1234567890");
}

#[tokio::test]
async fn test_generate_password_with_only_special_chars() {
    let mock_service =
        Arc::new(MockUtilitiesService::new().with_password("!@#$%^&*()".to_string()));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 10,
        upper: false,
        lower: false,
        digits: false,
        special: true,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, "!@#$%^&*()");
}

#[tokio::test]
async fn test_generate_password_service_failure() {
    let mock_service = Arc::new(MockUtilitiesService::new().should_fail(true));
    let use_case = GeneratePasswordUseCase::new(mock_service);

    let input = GeneratePasswordInput {
        len: 16,
        upper: true,
        lower: true,
        digits: true,
        special: true,
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let password = result.unwrap();
    assert_eq!(password, ""); // Service failure returns empty string
}
