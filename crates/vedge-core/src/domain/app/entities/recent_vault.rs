use std::path::PathBuf;

use crate::domain::shared::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentVault {
    pub id: String,
    pub path: PathBuf,
    pub display_name: String,
    pub last_opened: Option<Timestamp>,
    pub sort_order: i32,
}
