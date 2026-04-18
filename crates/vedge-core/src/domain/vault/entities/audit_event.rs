use serde::{Deserialize, Serialize};

use crate::domain::shared::{DeviceId, EntryId, Timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AuditAction {
    Unlocked,
    Locked,
    Created,
    Viewed,
    Updated,
    Deleted,
    Restored,
    PermanentlyDeleted,
    Exported,
    PasswordChanged,
    TagCreated,
    TagRenamed,
    TagDeleted,
}

impl AuditAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditAction::Unlocked => "Unlocked",
            AuditAction::Locked => "Locked",
            AuditAction::Created => "Created",
            AuditAction::Viewed => "Viewed",
            AuditAction::Updated => "Updated",
            AuditAction::Deleted => "Deleted",
            AuditAction::Restored => "Restored",
            AuditAction::PermanentlyDeleted => "PermanentlyDeleted",
            AuditAction::Exported => "Exported",
            AuditAction::PasswordChanged => "PasswordChanged",
            AuditAction::TagCreated => "TagCreated",
            AuditAction::TagRenamed => "TagRenamed",
            AuditAction::TagDeleted => "TagDeleted",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "Unlocked" => AuditAction::Unlocked,
            "Locked" => AuditAction::Locked,
            "Created" => AuditAction::Created,
            "Viewed" => AuditAction::Viewed,
            "Updated" => AuditAction::Updated,
            "Deleted" => AuditAction::Deleted,
            "Restored" => AuditAction::Restored,
            "PermanentlyDeleted" => AuditAction::PermanentlyDeleted,
            "Exported" => AuditAction::Exported,
            "PasswordChanged" => AuditAction::PasswordChanged,
            "TagCreated" => AuditAction::TagCreated,
            "TagRenamed" => AuditAction::TagRenamed,
            "TagDeleted" => AuditAction::TagDeleted,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: String,
    pub entry_id: Option<EntryId>,
    pub action: AuditAction,
    pub occurred_at: Timestamp,
    pub device_id: Option<DeviceId>,
}
