use crate::domain::shared::{ThemeId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub id: ThemeId,
    pub name: String,
    pub is_built_in: bool,
    pub root_background: String,
    pub root_foreground: String,
    pub root_primary: String,
    pub danger_base: Option<String>,
    pub warning_base: Option<String>,
    pub success_base: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
