//! Audit-log DTO converters (slice 4.1).
//!
//! `action` crosses as the raw `PascalCase` name (`AuditAction::as_str`); the
//! app owns localization. A *stored* action string this build can't parse is
//! degraded leniently in the query path, but an action *filter* naming a
//! nonexistent action is a client bug → `CommandError::Invalid`.

pub use vedge_ipc::{AuditEventDto, AuditPageDto, AuditQueryDto};

use vedge_core::domain::shared::EntryId;
use vedge_core::{AuditAction, AuditEvent, AuditQuery};

use crate::dto::common::{ts_from_string, ts_to_string};
use crate::error::CommandError;

#[must_use]
pub fn audit_event_to_dto(e: &AuditEvent) -> AuditEventDto {
    AuditEventDto {
        id: e.id.clone(),
        entry_id: e.entry_id.as_ref().map(|i| i.as_str().to_owned()),
        action: e.action.as_str().to_owned(),
        occurred_at: ts_to_string(e.occurred_at),
        device_id: e.device_id.as_ref().map(|d| d.as_str().to_owned()),
    }
}

pub fn audit_query_from_dto(d: &AuditQueryDto) -> Result<AuditQuery, CommandError> {
    let actions = d
        .actions
        .iter()
        .map(|a| {
            AuditAction::parse(a)
                .ok_or_else(|| CommandError::Invalid(format!("unknown audit action: {a}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AuditQuery {
        entry_id: d.entry_id.clone().map(EntryId::from_raw),
        actions,
        since: d.since.as_deref().map(ts_from_string).transpose()?,
        until: d.until.as_deref().map(ts_from_string).transpose()?,
        limit: d.limit,
        offset: d.offset,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use vedge_core::domain::shared::now;

    fn query(actions: Vec<String>) -> AuditQueryDto {
        AuditQueryDto {
            entry_id: None,
            actions,
            since: None,
            until: None,
            limit: 50,
            offset: 0,
        }
    }

    #[test]
    fn audit_query_from_dto_rejects_unknown_action() {
        let err = audit_query_from_dto(&query(vec!["FutureAction".into()])).unwrap_err();
        assert!(matches!(err, CommandError::Invalid(_)));
    }

    #[test]
    fn audit_query_from_dto_parses_timestamps() {
        let q = audit_query_from_dto(&AuditQueryDto {
            entry_id: Some("01HENTRY".into()),
            actions: vec!["Viewed".into()],
            since: Some("2026-01-01T00:00:00.000Z".into()),
            until: Some("2026-02-01T00:00:00.000Z".into()),
            limit: 25,
            offset: 0,
        })
        .unwrap();
        assert!(q.since.is_some());
        assert!(q.until.is_some());
        assert_eq!(q.entry_id.as_ref().map(EntryId::as_str), Some("01HENTRY"));
    }

    #[test]
    fn audit_action_wire_names_match_core_and_round_trip() {
        // The shared ipc `ACTION_NAMES` table is pinned one-for-one to the core
        // enum (order + count), so a new `AuditAction` can't silently drift out of
        // the wire vocabulary or the app's filter (which derives from it). The
        // core `as_str`/`parse` matches are the compile gate on the enum itself;
        // this guards the *derived* list. Each variant also round-trips
        // name → DTO → parsed action losslessly.
        assert_eq!(AuditAction::ALL.len(), vedge_ipc::ACTION_NAMES.len());
        for (a, name) in AuditAction::ALL.into_iter().zip(vedge_ipc::ACTION_NAMES) {
            assert_eq!(
                a.as_str(),
                name,
                "ACTION_NAMES is out of sync with AuditAction"
            );
            let event = AuditEvent {
                id: "x".into(),
                entry_id: None,
                action: a.clone(),
                occurred_at: now(),
                device_id: None,
            };
            let dto = audit_event_to_dto(&event);
            let q = audit_query_from_dto(&query(vec![dto.action.clone()])).unwrap();
            assert_eq!(q.actions, vec![a]);
        }
    }
}
