#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationInput {
    pub limit: i32,
    pub offset: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationMetadata {
    items_per_page: i32,
    total_count: i32,
    total_pages: i32,
    current_page: i32,
    next_page: i32,
    previous_page: i32,
    has_next_page: bool,
    has_previous_page: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationOutput<T> {
    pub data: Vec<T>,
    pub metadata: PaginationMetadata,
}
