use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct VaultItemDataCredential {
    pub identifier: String,
    pub password: String,
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind")]
pub enum VaultItemData {
    Credential(VaultItemDataCredential),
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct VaultItem {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub data: VaultItemData,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PaginationMetadata {
    pub total_count: u64,
    pub total_pages: u64,
    pub current_page: u64,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PaginationOutput<T> {
    pub data: Vec<T>,
    pub metadata: PaginationMetadata,
}
