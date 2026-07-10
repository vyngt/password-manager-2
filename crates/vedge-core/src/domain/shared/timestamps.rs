use chrono::{DateTime, Utc};

pub type Timestamp = DateTime<Utc>;

#[must_use]
pub fn now() -> Timestamp {
    Utc::now()
}

/// Format a `Timestamp` as fixed-width, millisecond-precision RFC-3339 UTC
/// (`YYYY-MM-DDTHH:MM:SS.sssZ`) — the single source of truth for stored and
/// wire timestamps.
///
/// Standing rule: this is the *only* formatter for `Timestamp`; every persisted
/// string is fixed-width so nothing downstream is tempted to lexically compare a
/// variable-width form. Comparisons themselves must still operate on the parsed
/// instant (or the typed `Timestamp`), never on the string. `SecondsFormat::Millis`
/// mirrors `vedge_ipc::ts_to_string` at the DTO boundary.
#[must_use]
pub fn format_rfc3339_millis(ts: Timestamp) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::format_rfc3339_millis;
    use chrono::TimeZone;

    #[test]
    fn format_is_fixed_width_millis_z() {
        // A whole-second instant must still carry `.000` and a `Z`, not `+00:00`.
        let ts = chrono::Utc
            .with_ymd_and_hms(2026, 1, 2, 3, 4, 5)
            .single()
            .unwrap();
        assert_eq!(format_rfc3339_millis(ts), "2026-01-02T03:04:05.000Z");
    }
}
