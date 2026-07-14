//! Slice 4.1 — audit-log read path (`query_audit` + `list_audit`).
//!
//! Real crypto + fast KDF via the shared `Harness`. Repo-level tests exercise
//! the SQL-side filtering/paging directly; use-case tests exercise the clamp and
//! the load-bearing audit-silence invariant.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use chrono::{DateTime, Utc};
use common::{Harness, build_unlock};
use secrecy::SecretString;

use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::domain::shared::{EntryId, Timestamp};
use vedge_core::domain::vault::entities::{AuditAction, AuditEvent, AuditQuery};
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, NotePayload};
use vedge_core::infrastructure::sqlite::vault::VaultDbConnection;
use vedge_core::{AUDIT_PAGE_DEFAULT, AUDIT_PAGE_MAX, CreateEntryInput, UnlockVaultInput};
use vedge_core::{GetEntryInput, create_entry, get_entry, list_audit};

fn ts(s: &str) -> Timestamp {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn ev(action: AuditAction, entry_id: Option<EntryId>, occurred_at: Timestamp) -> AuditEvent {
    AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id,
        action,
        occurred_at,
        device_id: None,
    }
}

async fn append_all(h: &Harness, events: &[AuditEvent]) {
    for e in events {
        h.repo.append_audit(e).await.unwrap();
    }
}

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn query_audit_orders_newest_first() {
    let h = Harness::fresh().await;
    append_all(
        &h,
        &[
            ev(AuditAction::Created, None, ts("2026-01-01T00:00:01.000Z")),
            ev(AuditAction::Viewed, None, ts("2026-01-01T00:00:02.000Z")),
            ev(AuditAction::Updated, None, ts("2026-01-01T00:00:03.000Z")),
        ],
    )
    .await;

    let page = h
        .repo
        .query_audit(&AuditQuery {
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page.total, 3);
    let actions: Vec<_> = page.events.iter().map(|e| e.action.clone()).collect();
    assert_eq!(
        actions,
        vec![
            AuditAction::Updated,
            AuditAction::Viewed,
            AuditAction::Created
        ]
    );
}

#[tokio::test]
async fn query_audit_filters_by_action_in_sql() {
    let h = Harness::fresh().await;
    append_all(
        &h,
        &[
            ev(AuditAction::Viewed, None, ts("2026-01-01T00:00:01.000Z")),
            ev(AuditAction::Updated, None, ts("2026-01-01T00:00:02.000Z")),
            ev(AuditAction::Viewed, None, ts("2026-01-01T00:00:03.000Z")),
            ev(AuditAction::Created, None, ts("2026-01-01T00:00:04.000Z")),
            ev(AuditAction::Viewed, None, ts("2026-01-01T00:00:05.000Z")),
        ],
    )
    .await;

    let page = h
        .repo
        .query_audit(&AuditQuery {
            actions: vec![AuditAction::Viewed],
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    // The regression test for Correction #3: `total` counts only matching rows,
    // not the whole log — filtering constrains the query, not the page.
    assert_eq!(page.total, 3);
    assert_eq!(page.events.len(), 3);
    assert!(page.events.iter().all(|e| e.action == AuditAction::Viewed));
}

#[tokio::test]
async fn query_audit_filters_by_entry() {
    let h = Harness::fresh().await;
    let target = EntryId::new();
    let other = EntryId::new();
    append_all(
        &h,
        &[
            ev(
                AuditAction::Created,
                Some(target.clone()),
                ts("2026-01-01T00:00:01.000Z"),
            ),
            ev(
                AuditAction::Viewed,
                Some(other.clone()),
                ts("2026-01-01T00:00:02.000Z"),
            ),
            // vault-level event: entry_id = NULL, must be excluded
            ev(AuditAction::Unlocked, None, ts("2026-01-01T00:00:03.000Z")),
            ev(
                AuditAction::Updated,
                Some(target.clone()),
                ts("2026-01-01T00:00:04.000Z"),
            ),
        ],
    )
    .await;

    let page = h
        .repo
        .query_audit(&AuditQuery {
            entry_id: Some(target.clone()),
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page.total, 2);
    assert!(
        page.events
            .iter()
            .all(|e| e.entry_id.as_ref() == Some(&target))
    );
}

#[tokio::test]
async fn query_audit_filters_by_date_range() {
    let h = Harness::fresh().await;
    let t1 = ts("2026-01-01T00:00:01.000Z");
    let t2 = ts("2026-01-01T00:00:02.000Z");
    let t3 = ts("2026-01-01T00:00:03.000Z");
    append_all(
        &h,
        &[
            ev(AuditAction::Created, None, t1),
            ev(AuditAction::Viewed, None, t2),
            ev(AuditAction::Updated, None, t3),
        ],
    )
    .await;

    // since inclusive (t2 in), until exclusive (t3 out) → only t2.
    let page = h
        .repo
        .query_audit(&AuditQuery {
            since: Some(t2),
            until: Some(t3),
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(page.total, 1);
    assert_eq!(page.events.len(), 1);
    assert_eq!(page.events[0].action, AuditAction::Viewed);
}

#[tokio::test]
async fn query_audit_paginates() {
    let h = Harness::fresh().await;
    let base = ts("2026-01-01T00:00:00.000Z");
    let events: Vec<AuditEvent> = (0..120)
        .map(|i| {
            ev(
                AuditAction::Viewed,
                None,
                base + chrono::Duration::milliseconds(i),
            )
        })
        .collect();
    append_all(&h, &events).await;

    let first = h
        .repo
        .query_audit(&AuditQuery {
            limit: 50,
            offset: 0,
            ..Default::default()
        })
        .await
        .unwrap();
    let second = h
        .repo
        .query_audit(&AuditQuery {
            limit: 50,
            offset: 50,
            ..Default::default()
        })
        .await
        .unwrap();
    let third = h
        .repo
        .query_audit(&AuditQuery {
            limit: 50,
            offset: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(first.total, 120);
    assert_eq!(second.total, 120);
    assert_eq!(third.total, 120);
    assert_eq!(first.events.len(), 50);
    assert_eq!(second.events.len(), 50);
    assert_eq!(third.events.len(), 20);

    // Pages are disjoint (offset advances through a stable desc order).
    let ids_first: Vec<_> = first.events.iter().map(|e| e.id.clone()).collect();
    let ids_second: Vec<_> = second.events.iter().map(|e| e.id.clone()).collect();
    assert!(ids_first.iter().all(|id| !ids_second.contains(id)));
}

#[tokio::test]
async fn query_audit_paginates_stably_on_equal_timestamps() {
    // Every row shares one millisecond — the ULID-id tiebreak must still give a
    // total order so each row lands on exactly one page (no dup, no skip).
    let h = Harness::fresh().await;
    let same = ts("2026-01-01T00:00:00.000Z");
    let events: Vec<AuditEvent> = (0..120)
        .map(|_| ev(AuditAction::Viewed, None, same))
        .collect();
    append_all(&h, &events).await;

    let mut seen = std::collections::HashSet::new();
    for offset in [0_u32, 50, 100] {
        let page = h
            .repo
            .query_audit(&AuditQuery {
                limit: 50,
                offset,
                ..Default::default()
            })
            .await
            .unwrap();
        for e in page.events {
            assert!(
                seen.insert(e.id),
                "a row appeared on two pages (unstable order)"
            );
        }
    }
    assert_eq!(
        seen.len(),
        120,
        "every row must appear exactly once across the pages"
    );
}

#[tokio::test]
async fn query_audit_skips_unparseable_action() {
    let h = Harness::fresh().await;
    append_all(
        &h,
        &[
            ev(AuditAction::Created, None, ts("2026-01-01T00:00:01.000Z")),
            ev(AuditAction::Viewed, None, ts("2026-01-01T00:00:03.000Z")),
        ],
    )
    .await;

    // Hand-insert a row whose action this build can't parse (simulates a
    // variant added by a later slice). A second connection to the same .vdb.
    let db2 = VaultDbConnection::open(&h.home.join(vedge_core::domain::shared::VAULT_FILE))
        .await
        .unwrap();
    db2.handle()
        .as_ref()
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "INSERT INTO audit_log (id, entry_id, action, occurred_at, device_id) \
             VALUES ('01JUNPARSE0000000000000000', NULL, 'FutureAction', \
             '2026-01-01T00:00:02.000Z', NULL)"
                .to_owned(),
        ))
        .await
        .unwrap();

    let page = h
        .repo
        .query_audit(&AuditQuery {
            limit: 100,
            ..Default::default()
        })
        .await
        .unwrap();

    // The two parseable rows are returned; the page does NOT error. The
    // hand-inserted unparseable row is skipped, not surfaced.
    assert_eq!(page.events.len(), 2);
    assert!(
        page.events
            .iter()
            .all(|e| e.id != "01JUNPARSE0000000000000000")
    );
    // Documented caveat: `total` (COUNT(*)) still includes the skipped row.
    assert_eq!(page.total, 3);
}

#[tokio::test]
async fn list_audit_clamps_limit() {
    let h = Harness::fresh().await;
    let session = unlock(&h).await;
    // Unlock appends its own `Unlocked` row; count it as the baseline.
    let base_count = h.repo.recent_audit(1_000_000).await.unwrap().len() as u64;

    let base = ts("2026-01-01T00:00:00.000Z");
    let events: Vec<AuditEvent> = (0..(AUDIT_PAGE_MAX + 5))
        .map(|i| {
            ev(
                AuditAction::Viewed,
                None,
                base + chrono::Duration::milliseconds(i64::from(i)),
            )
        })
        .collect();
    append_all(&h, &events).await;

    // limit = 0 clamps up to 1.
    let low = list_audit(
        &session,
        AuditQuery {
            limit: 0,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(low.events.len(), 1);

    // limit = 9999 clamps down to AUDIT_PAGE_MAX.
    let high = list_audit(
        &session,
        AuditQuery {
            limit: 9999,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(high.events.len(), AUDIT_PAGE_MAX as usize);
    assert_eq!(high.total, base_count + u64::from(AUDIT_PAGE_MAX) + 5);
}

#[tokio::test]
async fn list_audit_is_audit_silent() {
    // The load-bearing test: listing the audit trail must write no audit rows
    // and bump no `accessed_at`.
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: EntryPayload::Note(NotePayload {
                meta: CommonMeta::new("n", EntryType::Note),
                content: SecretString::from("secret"),
            }),
        },
    )
    .await
    .unwrap()
    .entry_id;
    // View it once so there's an `accessed_at` value and a `Viewed` row to guard.
    get_entry(&mut session, GetEntryInput { entry_id: id })
        .await
        .unwrap();

    let audit_before = h.repo.recent_audit(100_000).await.unwrap().len();
    let mut accessed_before: Vec<(String, Option<Timestamp>)> = h
        .repo
        .all_entries()
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.id.as_str().to_owned(), r.accessed_at))
        .collect();
    accessed_before.sort();

    // Read the trail a couple of times.
    for _ in 0..2 {
        list_audit(
            &session,
            AuditQuery {
                limit: AUDIT_PAGE_DEFAULT,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    let audit_after = h.repo.recent_audit(100_000).await.unwrap().len();
    let mut accessed_after: Vec<(String, Option<Timestamp>)> = h
        .repo
        .all_entries()
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.id.as_str().to_owned(), r.accessed_at))
        .collect();
    accessed_after.sort();

    assert_eq!(audit_after, audit_before, "list_audit wrote audit rows");
    assert_eq!(
        accessed_after, accessed_before,
        "list_audit bumped accessed_at"
    );
}
