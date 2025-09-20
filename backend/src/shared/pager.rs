#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationInput {
    pub limit: u64,
    pub offset: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationMetadata {
    items_per_page: u64,
    total_count: u64,
    total_pages: u64,
    current_page: u64,
    next_page: u64,
    previous_page: u64,
    has_next_page: bool,
    has_previous_page: bool,
}

impl PaginationMetadata {
    pub fn calculate_pagination(limit: u64, offset: u64, total_count: u64) -> Self {
        let total_pages = (total_count + limit - 1) / limit; // ceil
        let current_page = (offset / limit) + 1;

        let next_page = if current_page < total_pages {
            current_page + 1
        } else {
            current_page
        };

        let previous_page = if current_page > 1 {
            current_page - 1
        } else {
            current_page
        };

        let has_next_page = current_page < total_pages;
        let has_previous_page = current_page > 1;

        return PaginationMetadata {
            items_per_page: limit,
            total_count: total_count,
            total_pages: total_pages,
            current_page: current_page,
            next_page: next_page,
            previous_page: previous_page,
            has_next_page: has_next_page,
            has_previous_page: has_previous_page,
        };
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginationOutput<T> {
    pub data: Vec<T>,
    pub metadata: PaginationMetadata,
}
