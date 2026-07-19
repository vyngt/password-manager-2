//! Ordering keys for DTO timestamp strings.
//!
//! **Standing rule:** timestamp comparisons operate on the *parsed instant*,
//! never on the DTO string. The `IndexEntryDto` / `RegisteredVaultDto` timestamps
//! cross the boundary as RFC-3339 text ([`vedge_ipc::ts_to_string`] → fixed-width
//! millis-`Z`), but nothing type-enforces that width, so a lexical `str` compare
//! is not guaranteed chronological (`"…+02:00"` sorts after `"…Z"` textually yet
//! is the *earlier* instant). Parse first, compare the `i64` millis. The core
//! domain keeps its comparisons on the typed `Timestamp` (`chrono::DateTime<Utc>`,
//! correct `Ord` by construction) — this helper only exists for the DTO layer.

use chrono::DateTime;

/// Parse an RFC-3339 timestamp into epoch **milliseconds** for ordering.
///
/// Robust to fractional-second width and `Z` vs `+NN:NN` — it compares the
/// absolute instant, not the text. An unparseable (or, at the call sites,
/// missing) stamp maps to [`i64::MIN`] so it sorts oldest deterministically and
/// never panics. Millis precision is lossless here: the DTO boundary is already
/// millis ([`vedge_ipc::ts_to_string`] uses `SecondsFormat::Millis`).
pub fn ts_millis(s: &str) -> i64 {
    DateTime::parse_from_rfc3339(s).map_or(i64::MIN, |dt| dt.timestamp_millis())
}

/// True if `rfc3339` is more than `days` old relative to `now_ms` (epoch millis) — the
/// credential-age nudge threshold (slice 5.9 ③). The caller supplies `now_ms`
/// (`js_sys::Date::now()`), keeping this pure/testable, the way `relative_time` does.
///
/// 🔴 An unparseable/missing stamp (`ts_millis` → `i64::MIN`) is treated as NOT stale: the nudge
/// must never fire on a value it could not read (fail safe — a false "your password is old" alarm
/// trains users to click through warnings).
#[must_use]
pub fn is_older_than_days(rfc3339: &str, now_ms: i64, days: i64) -> bool {
    let ts = ts_millis(rfc3339);
    if ts == i64::MIN {
        return false;
    }
    now_ms.saturating_sub(ts) > days.saturating_mul(86_400_000)
}

#[cfg(test)]
mod tests {
    use super::{is_older_than_days, ts_millis};

    const DAY_MS: i64 = 86_400_000;

    #[test]
    fn older_than_threshold_is_stale() {
        let now = 400 * DAY_MS;
        // Changed at t=0 → 400 days old.
        let changed = "1970-01-01T00:00:00.000Z";
        assert!(is_older_than_days(changed, now, 365));
        assert!(!is_older_than_days(changed, now, 730));
    }

    #[test]
    fn unreadable_stamp_is_never_stale() {
        assert!(!is_older_than_days("not a timestamp", i64::MAX, 365));
    }

    #[test]
    fn parses_z_and_offset_to_same_instant() {
        // 10:00 UTC written two ways must yield the same ordering key…
        assert_eq!(
            ts_millis("2026-07-05T10:00:00.000Z"),
            ts_millis("2026-07-05T12:00:00.000+02:00")
        );
    }

    #[test]
    fn offset_orders_by_instant_not_text() {
        // "12:…+02:00" (10:00Z) is textually greater but chronologically earlier
        // than "11:…Z" (11:00Z).
        let later_text_earlier_instant = ts_millis("2026-07-05T12:00:00.000+02:00");
        let earlier_text_later_instant = ts_millis("2026-07-05T11:00:00.000Z");
        assert!(earlier_text_later_instant > later_text_earlier_instant);
    }

    #[test]
    fn unparseable_is_min_and_total() {
        assert_eq!(ts_millis("not a timestamp"), i64::MIN);
        assert_eq!(ts_millis(""), i64::MIN);
    }
}
