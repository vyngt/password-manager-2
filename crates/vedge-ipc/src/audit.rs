//! Audit-log wire DTOs (slice 4.1).
//!
//! `action` crosses as the raw `PascalCase` string (`AuditAction::as_str()`),
//! not a typed enum: the DB already stores TEXT, the app owns localization, and
//! a raw string is inherently forward-compatible — a variant added by a later
//! slice deserializes fine and the app maps an unrecognized string to a muted
//! chip. Query predicates are all applied SQL-side in the core, so the page
//! composes correctly with filtering (see the 4.1 spec).
//!
//! `snake_case` on the wire is achieved by field naming (no `rename_all`), like
//! every other DTO in this crate; optionals carry `#[serde(default)]`.

use serde::{Deserialize, Serialize};

/// The canonical `AuditAction` names in **core-enum order**.
///
/// The single wire-format source of truth shared across the IPC boundary: the
/// app's audit filter derives its dropdown from this (so it can't drift), and a
/// `vedge-tauri` test pins it one-for-one to `vedge_core::AuditAction::ALL` (so a
/// new core variant can't silently drift out of it). Extend both together.
pub const ACTION_NAMES: [&str; 19] = [
    "Unlocked",
    "Locked",
    "Created",
    "Viewed",
    "Updated",
    "Deleted",
    "Restored",
    "PermanentlyDeleted",
    "Exported",
    "PasswordChanged",
    "TagCreated",
    "TagRenamed",
    "TagDeleted",
    "RecoveryUsed",
    "BiometricUnlocked",
    "TotpRevealed",
    "HealthScanned",
    "BackupCreated",
    "BackupRestored",
];

/// Filter + page parameters for `list_audit`. Empty `actions` = all.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryDto {
    #[serde(default)]
    pub entry_id: Option<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    /// Inclusive lower bound, RFC-3339.
    #[serde(default)]
    pub since: Option<String>,
    /// Exclusive upper bound, RFC-3339.
    #[serde(default)]
    pub until: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

/// One audit-trail event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEventDto {
    pub id: String,
    #[serde(default)]
    pub entry_id: Option<String>,
    /// Raw `PascalCase` action name (`AuditAction::as_str()`); the app localizes.
    pub action: String,
    /// RFC-3339 millis-`Z`.
    pub occurred_at: String,
    /// Always `None` today — carried for forward-compat, not rendered.
    #[serde(default)]
    pub device_id: Option<String>,
}

/// One page of the audit trail plus the total match count (ignoring limit/offset).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPageDto {
    pub events: Vec<AuditEventDto>,
    pub total: u64,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn audit_query_dto_round_trips() {
        let dto = AuditQueryDto {
            entry_id: Some("01HENTRY".into()),
            actions: vec!["Viewed".into(), "Updated".into()],
            since: Some("2026-01-01T00:00:00.000Z".into()),
            until: None,
            limit: 50,
            offset: 100,
        };
        let s = serde_json::to_string(&dto).unwrap();
        let back: AuditQueryDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back.entry_id, dto.entry_id);
        assert_eq!(back.actions, dto.actions);
        assert_eq!(back.since, dto.since);
        assert_eq!(back.until, dto.until);
        assert_eq!(back.limit, dto.limit);
        assert_eq!(back.offset, dto.offset);
    }

    #[test]
    fn audit_query_dto_defaults_from_partial_json() {
        // Only limit/offset are required; every optional defaults.
        let dto: AuditQueryDto = serde_json::from_str(r#"{"limit":25,"offset":0}"#).unwrap();
        assert!(dto.entry_id.is_none());
        assert!(dto.actions.is_empty());
        assert!(dto.since.is_none());
        assert!(dto.until.is_none());
        assert_eq!(dto.limit, 25);
        assert_eq!(dto.offset, 0);
    }

    #[test]
    fn audit_event_dto_round_trips() {
        let dto = AuditEventDto {
            id: "01HEVENT".into(),
            entry_id: Some("01HENTRY".into()),
            action: "Viewed".into(),
            occurred_at: "2026-07-11T09:14:03.000Z".into(),
            device_id: None,
        };
        let s = serde_json::to_string(&dto).unwrap();
        let back: AuditEventDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back, dto);
    }

    #[test]
    fn audit_page_dto_round_trips() {
        let dto = AuditPageDto {
            events: vec![AuditEventDto {
                id: "01HEVENT".into(),
                entry_id: None,
                action: "Unlocked".into(),
                occurred_at: "2026-07-11T09:14:03.000Z".into(),
                device_id: None,
            }],
            total: 12_431,
        };
        let s = serde_json::to_string(&dto).unwrap();
        let back: AuditPageDto = serde_json::from_str(&s).unwrap();
        assert_eq!(back, dto);
    }
}
