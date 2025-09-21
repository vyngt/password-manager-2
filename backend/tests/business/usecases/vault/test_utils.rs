use async_trait::async_trait;
use backend::business::domain::entities::vault_item::{
    UpdateVaultItem, VaultItem, VaultItemData, VaultItemDataCredential, VaultItemKind,
};
use backend::business::domain::repositories::{ListVaultItemsOptions, VaultItemRepository};
use backend::business::domain::services::VaultService;
use backend::errors::AppResult;
use backend::shared::pager::{PaginationInput, PaginationOutput};
use uuid::Uuid;

// Mock implementation of VaultItemRepository for testing
pub struct MockVaultItemRepository {
    pub vault_items: Vec<VaultItem>,
    pub should_fail: bool,
}

impl MockVaultItemRepository {
    pub fn new() -> Self {
        Self {
            vault_items: Vec::new(),
            should_fail: false,
        }
    }

    pub fn with_vault_items(mut self, items: Vec<VaultItem>) -> Self {
        self.vault_items = items;
        self
    }

    pub fn should_fail(mut self, fail: bool) -> Self {
        self.should_fail = fail;
        self
    }
}

#[async_trait]
impl VaultItemRepository for MockVaultItemRepository {
    async fn list(&self, input: ListVaultItemsOptions) -> AppResult<PaginationOutput<VaultItem>> {
        if self.should_fail {
            return Err(backend::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        let total_count = self.vault_items.len() as u64;
        let output = PaginationOutput {
            data: self.vault_items.clone(),
            metadata: backend::shared::pager::PaginationMetadata::calculate_pagination(
                input.pagination.limit,
                input.pagination.offset,
                total_count,
            ),
        };

        Ok(output)
    }

    async fn get(&self, id: Uuid) -> AppResult<VaultItem> {
        if self.should_fail {
            return Err(backend::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        self.vault_items
            .iter()
            .find(|item| item.id == id)
            .cloned()
            .ok_or_else(|| {
                backend::errors::AppError::DataOperationError(
                    backend::errors::DataOperation::NotFoundError,
                )
            })
    }

    async fn create(&self, item: VaultItem) -> AppResult<VaultItem> {
        if self.should_fail {
            return Err(backend::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }
        Ok(item)
    }

    async fn update(&self, id: Uuid, update_item: UpdateVaultItem) -> AppResult<VaultItem> {
        if self.should_fail {
            return Err(backend::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        if let Some(mut item) = self.vault_items.iter().find(|item| item.id == id).cloned() {
            item.title = update_item.title;
            item.kind = update_item.kind;
            item.data = update_item.data;
            Ok(item)
        } else {
            Err(backend::errors::AppError::DataOperationError(
                backend::errors::DataOperation::NotFoundError,
            ))
        }
    }

    async fn delete(&self, id: Uuid) -> AppResult<VaultItem> {
        if self.should_fail {
            return Err(backend::errors::AppError::UnknownError(
                "Mock error".to_string(),
            ));
        }

        // For delete, we return the actual item if found, or create a mock item with the requested ID
        if let Some(item) = self.vault_items.iter().find(|item| item.id == id) {
            Ok(item.clone())
        } else {
            // If not found in the list, return a mock item with the requested ID
            // This simulates the behavior where delete returns the deleted item
            let mock_item = VaultItem::new(
                Some(id),
                "Deleted Item".to_string(),
                VaultItemKind::Credential,
                VaultItemData::Credential(VaultItemDataCredential {
                    identifier: "deleted@example.com".to_string(),
                    password: "deleted".to_string(),
                    url: "https://deleted.com".to_string(),
                }),
            );
            Ok(mock_item)
        }
    }
}

pub fn create_test_vault_item() -> VaultItem {
    VaultItem::new(
        None,
        "Test Vault Item".to_string(),
        VaultItemKind::Credential,
        VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    )
}

pub fn create_test_vault_item_with_id(id: Uuid) -> VaultItem {
    VaultItem::new(
        Some(id),
        "Test Vault Item".to_string(),
        VaultItemKind::Credential,
        VaultItemData::Credential(VaultItemDataCredential {
            identifier: "test@example.com".to_string(),
            password: "password123".to_string(),
            url: "https://example.com".to_string(),
        }),
    )
}

pub fn create_test_pagination_input() -> PaginationInput {
    PaginationInput {
        limit: 10,
        offset: 0,
    }
}

// Mock implementation of VaultService for testing
pub struct MockVaultService {
    pub should_unlock: bool,
    pub should_change_key: bool,
    pub should_fail: bool,
}

impl MockVaultService {
    pub fn new() -> Self {
        Self {
            should_unlock: true,
            should_change_key: true,
            should_fail: false,
        }
    }

    pub fn with_unlock_result(mut self, should_unlock: bool) -> Self {
        self.should_unlock = should_unlock;
        self
    }

    pub fn with_change_key_result(mut self, should_change_key: bool) -> Self {
        self.should_change_key = should_change_key;
        self
    }

    pub fn should_fail(mut self, fail: bool) -> Self {
        self.should_fail = fail;
        self
    }
}

#[async_trait]
impl VaultService for MockVaultService {
    async fn unlock(&self, _key: &str) -> bool {
        if self.should_fail {
            return false;
        }
        self.should_unlock
    }

    async fn change_key(&self, _new_key: &str) -> bool {
        if self.should_fail {
            return false;
        }
        self.should_change_key
    }
}
