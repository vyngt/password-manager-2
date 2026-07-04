use serde::Deserialize;

// NOTE: the legacy `VaultItem` / `VaultItemData` shim was retired in slice 1.5
// once the list and add-entry paths moved to `vedge_ipc::IndexEntryDto` /
// `PayloadDto`. The pagination types below are not yet wired.

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
