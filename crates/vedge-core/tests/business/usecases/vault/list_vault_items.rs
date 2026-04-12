use vedge_core::business::usecases::{ListVaultItemUseCase, ListVaultItemInput};
use vedge_core::shared::base::BaseUseCase;
use std::sync::Arc;

use crate::business::usecases::vault::test_utils::{
    create_test_vault_item, create_test_pagination_input, MockVaultItemRepository,
};

#[tokio::test]
async fn test_list_vault_items_success() {
    let item1 = create_test_vault_item();
    let item2 = create_test_vault_item();
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![item1.clone(), item2.clone()])
    );
    let use_case = ListVaultItemUseCase::new(mock_repo);

    let input = ListVaultItemInput {
        pagination: create_test_pagination_input(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let pagination_output = result.unwrap();
    assert_eq!(pagination_output.data.len(), 2);
    assert_eq!(pagination_output.metadata.total_count, 2);
    assert_eq!(pagination_output.metadata.current_page, 1);
    assert_eq!(pagination_output.metadata.items_per_page, 10);
    assert_eq!(pagination_output.metadata.total_pages, 1);
}

#[tokio::test]
async fn test_list_vault_items_empty_list() {
    let mock_repo = Arc::new(MockVaultItemRepository::new());
    let use_case = ListVaultItemUseCase::new(mock_repo);

    let input = ListVaultItemInput {
        pagination: create_test_pagination_input(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let pagination_output = result.unwrap();
    assert_eq!(pagination_output.data.len(), 0);
    assert_eq!(pagination_output.metadata.total_count, 0);
    assert_eq!(pagination_output.metadata.current_page, 1);
    assert_eq!(pagination_output.metadata.items_per_page, 10);
    assert_eq!(pagination_output.metadata.total_pages, 1);
}

#[tokio::test]
async fn test_list_vault_items_with_custom_pagination() {
    let items: Vec<_> = (0..25).map(|_| create_test_vault_item()).collect();
    let mock_repo = Arc::new(MockVaultItemRepository::new().with_vault_items(items));
    let use_case = ListVaultItemUseCase::new(mock_repo);

    let input = ListVaultItemInput {
        pagination: vedge_core::shared::pager::PaginationInput {
            limit: 10,
            offset: 10,
        },
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let pagination_output = result.unwrap();
    assert_eq!(pagination_output.data.len(), 25);
    assert_eq!(pagination_output.metadata.total_count, 25);
    assert_eq!(pagination_output.metadata.current_page, 2); // With offset 10 and limit 10, page should be 2
    assert_eq!(pagination_output.metadata.items_per_page, 10);
    assert_eq!(pagination_output.metadata.total_pages, 3); // 25 items with limit 10 = 3 pages
}

#[tokio::test]
async fn test_list_vault_items_repository_error() {
    let mock_repo = Arc::new(MockVaultItemRepository::new().should_fail(true));
    let use_case = ListVaultItemUseCase::new(mock_repo);

    let input = ListVaultItemInput {
        pagination: create_test_pagination_input(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        vedge_core::errors::AppError::UnknownError(_) => (),
        _ => panic!("Expected UnknownError"),
    }
}

#[tokio::test]
async fn test_list_vault_items_with_single_item() {
    let item = create_test_vault_item();
    let mock_repo = Arc::new(
        MockVaultItemRepository::new().with_vault_items(vec![item.clone()])
    );
    let use_case = ListVaultItemUseCase::new(mock_repo);

    let input = ListVaultItemInput {
        pagination: create_test_pagination_input(),
    };

    let result = use_case.execute(input).await;

    assert!(result.is_ok());
    let pagination_output = result.unwrap();
    assert_eq!(pagination_output.data.len(), 1);
    assert_eq!(pagination_output.metadata.total_count, 1);
    assert_eq!(pagination_output.data[0].id, item.id);
    assert_eq!(pagination_output.data[0].title, item.title);
}
