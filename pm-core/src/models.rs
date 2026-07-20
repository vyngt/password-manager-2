pub mod core;
pub mod theme;

/**
 * Pagination helper struct
 */
#[derive(serde::Serialize, serde::Deserialize)]
pub struct WithCount<T> {
    pub result: Vec<T>,
    pub total: i64,
}
