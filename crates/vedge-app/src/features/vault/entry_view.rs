//! Presentation helpers shared by the entry list (`vault_table`) and detail
//! panel (`vault_detail`). `short_date` is pure/host-testable; the entry-type
//! label is localized (`vault.type_*`) so it needs the i18n context.

use crate::i18n::*;
use icondata as i;
use leptos_i18n::I18nContext;
use vedge_ipc::EntryTypeDto;

/// Localized label for an entry type, for the list/detail Type field.
pub fn type_label_i18n(i18n: I18nContext<Locale>, entry_type: &EntryTypeDto) -> String {
    match entry_type {
        EntryTypeDto::Login => t_string!(i18n, vault.type_login).to_string(),
        EntryTypeDto::Card => t_string!(i18n, vault.type_card).to_string(),
        EntryTypeDto::SshKey => t_string!(i18n, vault.type_ssh_key).to_string(),
        EntryTypeDto::ApiKey => t_string!(i18n, vault.type_api_key).to_string(),
        EntryTypeDto::EnvVars => t_string!(i18n, vault.type_env_vars).to_string(),
        EntryTypeDto::Note => t_string!(i18n, vault.type_note).to_string(),
        EntryTypeDto::Document => t_string!(i18n, vault.type_document).to_string(),
        EntryTypeDto::Identity => t_string!(i18n, vault.type_identity).to_string(),
        EntryTypeDto::Folder => t_string!(i18n, vault.type_folder).to_string(),
        EntryTypeDto::Unknown(_) => t_string!(i18n, vault.type_other).to_string(),
    }
}

/// Icon token for an entry type, for the list / command-palette rows.
#[must_use]
pub fn type_icon(entry_type: &EntryTypeDto) -> icondata::Icon {
    match entry_type {
        EntryTypeDto::Login => i::FaGlobeSolid,
        EntryTypeDto::Card => i::FaCreditCardSolid,
        EntryTypeDto::SshKey => i::FaTerminalSolid,
        EntryTypeDto::ApiKey => i::FaKeySolid,
        EntryTypeDto::EnvVars => i::FaCodeSolid,
        EntryTypeDto::Note => i::FaFileLinesSolid,
        EntryTypeDto::Document => i::FaFileSolid,
        EntryTypeDto::Identity => i::FaIdCardSolid,
        EntryTypeDto::Folder => i::FaFolderSolid,
        EntryTypeDto::Unknown(_) => i::FaCircleQuestionSolid,
    }
}

/// Trim an RFC-3339 timestamp to its date portion (`2026-07-05T…` → `2026-07-05`).
/// Returns the input unchanged if there's no `T` separator.
pub fn short_date(rfc3339: &str) -> String {
    rfc3339
        .split_once('T')
        .map_or_else(|| rfc3339.to_string(), |(date, _)| date.to_string())
}

/// Long calendar date for the history panel (`2026-07-03T…` → `Jul 3, 2026`).
/// Falls back to [`short_date`] if the timestamp doesn't parse.
pub fn long_date(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339).map_or_else(
        |_| short_date(rfc3339),
        |dt| dt.format("%b %-d, %Y").to_string(),
    )
}

/// Coarse "time ago" bucket for the history panel's sub-label. Pure so the
/// caller passes `now` (millis since epoch) — the component supplies it from
/// `chrono::Utc::now()`. The localized wording lives in the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelTime {
    JustNow,
    Today,
    Days(i64),
    Weeks(i64),
    Months(i64),
}

/// Bucket the age of `rfc3339` relative to `now_ms`. Unparseable input or a
/// future timestamp collapses to `JustNow`.
#[must_use]
pub fn relative_time(rfc3339: &str, now_ms: i64) -> RelTime {
    let then_ms =
        chrono::DateTime::parse_from_rfc3339(rfc3339).map_or(now_ms, |dt| dt.timestamp_millis());
    let secs = (now_ms - then_ms).max(0) / 1000;
    let days = secs / 86_400;
    if secs < 60 {
        RelTime::JustNow
    } else if days < 1 {
        RelTime::Today
    } else if days < 14 {
        RelTime::Days(days)
    } else if days < 60 {
        RelTime::Weeks(days / 7)
    } else {
        RelTime::Months(days / 30)
    }
}

/// Human-readable byte size for the Document detail view (e.g. `2.5 MB`).
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::{RelTime, human_size, long_date, relative_time, short_date};

    #[test]
    fn short_date_trims_time() {
        assert_eq!(short_date("2026-07-05T12:00:00.000Z"), "2026-07-05");
    }

    #[test]
    fn short_date_passes_through_without_separator() {
        assert_eq!(short_date("2026-07-05"), "2026-07-05");
    }

    #[test]
    fn human_size_scales_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn long_date_formats_calendar_style() {
        assert_eq!(long_date("2026-07-03T09:00:00+00:00"), "Jul 3, 2026");
        assert_eq!(long_date("2026-12-25T00:00:00Z"), "Dec 25, 2026");
    }

    #[test]
    fn long_date_falls_back_on_bad_input() {
        assert_eq!(long_date("not-a-date"), "not-a-date");
    }

    #[test]
    fn relative_time_buckets() {
        // Base: 2026-07-08T00:00:00Z.
        let now = 1_783_641_600_000_i64;
        let at = |ms: i64| now - ms;
        const MIN: i64 = 60_000;
        const HOUR: i64 = 60 * MIN;
        const DAY: i64 = 24 * HOUR;
        let iso = |ms: i64| {
            chrono::DateTime::from_timestamp_millis(ms)
                .unwrap()
                .to_rfc3339()
        };
        assert_eq!(relative_time(&iso(at(30_000)), now), RelTime::JustNow);
        assert_eq!(relative_time(&iso(at(3 * HOUR)), now), RelTime::Today);
        assert_eq!(relative_time(&iso(at(4 * DAY)), now), RelTime::Days(4));
        assert_eq!(relative_time(&iso(at(15 * DAY)), now), RelTime::Weeks(2));
        assert_eq!(relative_time(&iso(at(90 * DAY)), now), RelTime::Months(3));
    }

    #[test]
    fn relative_time_future_is_just_now() {
        let now = 1_783_641_600_000_i64;
        let future = chrono::DateTime::from_timestamp_millis(now + 10_000)
            .unwrap()
            .to_rfc3339();
        assert_eq!(relative_time(&future, now), RelTime::JustNow);
    }
}
