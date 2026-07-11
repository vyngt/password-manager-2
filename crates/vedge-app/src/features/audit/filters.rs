//! Audit-page filter state + query building. Pure and host-testable — the page
//! owns the signals, this owns the logic.

use chrono::NaiveDate;
use vedge_ipc::AuditQueryDto;

/// Rows per audit page. Mirrors core's `AUDIT_PAGE_DEFAULT`.
pub const AUDIT_PAGE_SIZE: u32 = 50;

/// Active filter facets. Dates are whole days (the `DatePicker`'s granularity);
/// [`AuditView::to_query`] converts them to RFC-3339 boundaries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditFilters {
    /// Raw `PascalCase` action name; `None` = all actions.
    pub action: Option<String>,
    /// Entry ULID; `None` = all entries (incl. vault-level events).
    pub entry_id: Option<String>,
    /// Inclusive lower-bound day.
    pub since: Option<NaiveDate>,
    /// Inclusive upper-bound day (converted to an exclusive next-day boundary).
    pub until: Option<NaiveDate>,
}

/// Filters plus the current page offset.
///
/// **Any filter change resets `offset` to 0** — otherwise a filter that shrinks
/// the result set strands you on a page that no longer exists. Page navigation
/// changes `offset` directly and does *not* go through the mutators.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditView {
    pub filters: AuditFilters,
    pub offset: u32,
}

impl AuditView {
    pub fn set_action(&mut self, action: Option<String>) {
        self.filters.action = action;
        self.offset = 0;
    }

    pub fn set_entry(&mut self, entry_id: Option<String>) {
        self.filters.entry_id = entry_id;
        self.offset = 0;
    }

    pub fn set_since(&mut self, since: Option<NaiveDate>) {
        self.filters.since = since;
        self.offset = 0;
    }

    pub fn set_until(&mut self, until: Option<NaiveDate>) {
        self.filters.until = until;
        self.offset = 0;
    }

    /// Build the wire query. Dates become RFC-3339 millis-`Z` day boundaries:
    /// `since` at that day's 00:00:00 (inclusive), `until` at the *next* day's
    /// 00:00:00 — so the whole `until` day is included, since the core treats
    /// `until` as exclusive.
    #[must_use]
    pub fn to_query(&self) -> AuditQueryDto {
        AuditQueryDto {
            entry_id: self.filters.entry_id.clone(),
            actions: self.filters.action.clone().into_iter().collect(),
            since: self.filters.since.map(day_start),
            until: self.filters.until.and_then(|d| d.succ_opt()).map(day_start),
            limit: AUDIT_PAGE_SIZE,
            offset: self.offset,
        }
    }
}

fn day_start(d: NaiveDate) -> String {
    format!("{}T00:00:00.000Z", d.format("%Y-%m-%d"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn to_query_resets_offset_on_filter_change() {
        let mut v = AuditView {
            offset: 7,
            ..Default::default()
        };
        v.set_action(Some("Viewed".into()));
        assert_eq!(v.offset, 0);
        assert_eq!(v.to_query().offset, 0);
        assert_eq!(v.to_query().actions, vec!["Viewed".to_owned()]);

        let mut v = AuditView {
            offset: 3,
            ..Default::default()
        };
        v.set_since(Some(ymd(2026, 7, 1)));
        assert_eq!(v.offset, 0);

        let mut v = AuditView {
            offset: 9,
            ..Default::default()
        };
        v.set_entry(Some("01HENTRY".into()));
        assert_eq!(v.offset, 0);
    }

    #[test]
    fn to_query_none_filters_are_empty() {
        let q = AuditView::default().to_query();
        assert!(q.actions.is_empty());
        assert!(q.entry_id.is_none());
        assert!(q.since.is_none());
        assert!(q.until.is_none());
        assert_eq!(q.limit, AUDIT_PAGE_SIZE);
        assert_eq!(q.offset, 0);
    }

    #[test]
    fn to_query_converts_date_range_to_rfc3339_bounds() {
        let mut v = AuditView::default();
        v.set_since(Some(ymd(2026, 7, 1)));
        v.set_until(Some(ymd(2026, 7, 11)));
        let q = v.to_query();
        assert_eq!(q.since.as_deref(), Some("2026-07-01T00:00:00.000Z"));
        // `until` day is inclusive → exclusive *next-day* boundary.
        assert_eq!(q.until.as_deref(), Some("2026-07-12T00:00:00.000Z"));
    }

    #[test]
    fn to_query_preserves_offset_when_only_page_changes() {
        // A page change sets `offset` directly (not via a mutator), so it survives.
        let v = AuditView {
            offset: 100,
            ..Default::default()
        };
        assert_eq!(v.to_query().offset, 100);
    }
}
