use crate::domain::shared::{SessionId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSession {
    pub session_id: SessionId,
    pub browser: String,
    pub profile_name: Option<String>,
    pub session_key: [u8; 32],
    pub created_at: Timestamp,
    pub last_active_at: Option<Timestamp>,
}
