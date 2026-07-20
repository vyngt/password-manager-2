use serde_json::Value;

use crate::domain::shared::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSetting {
    pub key: String,
    pub value: Value,
    pub updated_at: Timestamp,
}
