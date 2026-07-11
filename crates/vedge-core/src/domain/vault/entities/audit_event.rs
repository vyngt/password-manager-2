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
    /// A Login entry's TOTP code was revealed (slice 4.2). Audited once per
    /// (session, entry); period-boundary refreshes in the same session are
    /// silent. Only the derived code is exposed — never the seed.
    TotpRevealed,
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
            Self::TotpRevealed => "TotpRevealed",
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
            "TotpRevealed" => Self::TotpRevealed,
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

/// A filtered, paged read over `audit_log`.
///
/// Every predicate is applied in SQL — filtering in memory would break
/// pagination (a page of 100 rows filtered down to 3 is not a page of 3
/// results). `limit` is clamped by the use case, not here.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Scope to one entry; `None` = all events (including vault-level events
    /// whose `entry_id` is `NULL`).
    pub entry_id: Option<EntryId>,
    /// Empty = all actions.
    pub actions: Vec<AuditAction>,
    /// Inclusive lower bound on `occurred_at`.
    pub since: Option<Timestamp>,
    /// Exclusive upper bound on `occurred_at`.
    pub until: Option<Timestamp>,
    pub limit: u32,
    pub offset: u32,
}

/// One page of the audit trail plus the total match count.
#[derive(Debug, Clone)]
pub struct AuditPage {
    pub events: Vec<AuditEvent>,
    /// Total rows matching the filter, ignoring limit/offset — drives the
    /// "showing 1–50 of N" summary and the page count.
    pub total: u64,
}
