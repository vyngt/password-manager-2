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
    /// Vault was unlocked via the Emergency Kit recovery path instead of
    /// the keychain-stored Secret Key. Distinguishes recovery from normal
    /// unlocks in the audit timeline.
    RecoveryUsed,
    /// Vault was unlocked via a biometric gate (Windows Hello / Touch ID)
    /// releasing the stored KEK, instead of the master password. Distinguishes
    /// biometric unlocks from normal unlocks in the audit timeline.
    BiometricUnlocked,
}

impl AuditAction {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unlocked => "Unlocked",
            Self::Locked => "Locked",
            Self::Created => "Created",
            Self::Viewed => "Viewed",
            Self::Updated => "Updated",
            Self::Deleted => "Deleted",
            Self::Restored => "Restored",
            Self::PermanentlyDeleted => "PermanentlyDeleted",
            Self::Exported => "Exported",
            Self::PasswordChanged => "PasswordChanged",
            Self::TagCreated => "TagCreated",
            Self::TagRenamed => "TagRenamed",
            Self::TagDeleted => "TagDeleted",
            Self::RecoveryUsed => "RecoveryUsed",
            Self::BiometricUnlocked => "BiometricUnlocked",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "Unlocked" => Self::Unlocked,
            "Locked" => Self::Locked,
            "Created" => Self::Created,
            "Viewed" => Self::Viewed,
            "Updated" => Self::Updated,
            "Deleted" => Self::Deleted,
            "Restored" => Self::Restored,
            "PermanentlyDeleted" => Self::PermanentlyDeleted,
            "Exported" => Self::Exported,
            "PasswordChanged" => Self::PasswordChanged,
            "TagCreated" => Self::TagCreated,
            "TagRenamed" => Self::TagRenamed,
            "TagDeleted" => Self::TagDeleted,
            "RecoveryUsed" => Self::RecoveryUsed,
            "BiometricUnlocked" => Self::BiometricUnlocked,
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
