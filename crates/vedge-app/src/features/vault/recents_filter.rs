//! Pure client-side filter + recency sort for the vault picker (2.8.1).
//!
//! Kept Leptos-free so it's host-testable: the picker derives its visible
//! list from `filter_sort_recents(&recents, &query)` inside a `Memo`.

use crate::features::vault::timestamps::ts_millis;
use vedge_ipc::RecentVaultStatusDto;

/// Filter recents by a free-text query (matched against display name +
/// path, case-insensitive substring; an empty query keeps everything) and
/// sort for display:
///
/// 1. existing files before missing ones,
/// 2. then most-recent first (`last_opened` desc; never-opened last),
/// 3. then display name A→Z as a stable tiebreak.
#[must_use]
pub fn filter_sort_recents(
    items: &[RecentVaultStatusDto],
    query: &str,
) -> Vec<RecentVaultStatusDto> {
    let q = query.trim().to_lowercase();
    let mut out: Vec<RecentVaultStatusDto> = items
        .iter()
        .filter(|r| {
            q.is_empty()
                || r.vault.display_name.to_lowercase().contains(&q)
                || r.vault.path.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();

    out.sort_by(|a, b| {
        // 1. Existing files first (missing sink to the bottom).
        b.exists
            .cmp(&a.exists)
            // 2. Most-recent first. `last_opened` is an RFC-3339 DTO *string*, so
            //    it is parsed to the absolute instant before comparison — never
            //    lexically (a variable-width / non-`Z` form would sort by text).
            //    `None`/unparseable → `i64::MIN`, so comparing b-vs-a puts the
            //    newest first and never-opened last. See [`super::timestamps`].
            .then_with(|| recency_millis(b).cmp(&recency_millis(a)))
            // 3. Stable, human-friendly tiebreak.
            .then_with(|| {
                a.vault
                    .display_name
                    .to_lowercase()
                    .cmp(&b.vault.display_name.to_lowercase())
            })
    });
    out
}

/// `last_opened` parsed to epoch millis for recency ordering; never-opened /
/// unparseable → `i64::MIN` (oldest). See [`super::timestamps::ts_millis`].
fn recency_millis(r: &RecentVaultStatusDto) -> i64 {
    r.vault.last_opened.as_deref().map_or(i64::MIN, ts_millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vedge_ipc::RecentVaultDto;

    fn row(
        id: &str,
        name: &str,
        path: &str,
        last_opened: Option<&str>,
        exists: bool,
    ) -> RecentVaultStatusDto {
        RecentVaultStatusDto {
            vault: RecentVaultDto {
                id: id.into(),
                path: path.into(),
                display_name: name.into(),
                last_opened: last_opened.map(str::to_owned),
                sort_order: 0,
            },
            exists,
            openable: exists,
        }
    }

    fn ids(v: &[RecentVaultStatusDto]) -> Vec<&str> {
        v.iter().map(|r| r.vault.id.as_str()).collect()
    }

    #[test]
    fn empty_query_returns_all_sorted_by_recency() {
        let items = vec![
            row(
                "old",
                "Old",
                "/a.vdb",
                Some("2020-01-01T00:00:00+00:00"),
                true,
            ),
            row(
                "new",
                "New",
                "/b.vdb",
                Some("2022-01-01T00:00:00+00:00"),
                true,
            ),
            row(
                "mid",
                "Mid",
                "/c.vdb",
                Some("2021-01-01T00:00:00+00:00"),
                true,
            ),
        ];
        assert_eq!(ids(&filter_sort_recents(&items, "")), ["new", "mid", "old"]);
    }

    #[test]
    fn filters_by_name_case_insensitive() {
        let items = vec![
            row("work", "Work", "/w.vdb", None, true),
            row("home", "Home", "/h.vdb", None, true),
        ];
        assert_eq!(ids(&filter_sort_recents(&items, "WOR")), ["work"]);
    }

    #[test]
    fn filters_by_path_substring() {
        let items = vec![
            row("work", "Work", "D:\\vaults\\work.vdb", None, true),
            row("home", "Home", "C:\\home.vdb", None, true),
        ];
        // Query matches only the first vault's path.
        assert_eq!(ids(&filter_sort_recents(&items, "vaults")), ["work"]);
    }

    #[test]
    fn no_match_returns_empty() {
        let items = vec![row("work", "Work", "/w.vdb", None, true)];
        assert!(filter_sort_recents(&items, "zzz").is_empty());
    }

    #[test]
    fn missing_files_sort_last_even_if_newest() {
        let items = vec![
            row(
                "gone",
                "Gone",
                "/g.vdb",
                Some("2099-01-01T00:00:00+00:00"),
                false,
            ),
            row(
                "ok",
                "Ok",
                "/o.vdb",
                Some("2000-01-01T00:00:00+00:00"),
                true,
            ),
        ];
        // The existing vault wins despite a much older timestamp.
        assert_eq!(ids(&filter_sort_recents(&items, "")), ["ok", "gone"]);
    }

    #[test]
    fn never_opened_sort_after_opened() {
        let items = vec![
            row("never", "Never", "/n.vdb", None, true),
            row(
                "opened",
                "Opened",
                "/o.vdb",
                Some("2020-01-01T00:00:00+00:00"),
                true,
            ),
        ];
        assert_eq!(ids(&filter_sort_recents(&items, "")), ["opened", "never"]);
    }

    #[test]
    fn recency_orders_by_instant_not_text() {
        // Regression guard: `"12:…+02:00"` (10:00Z) is textually greater than
        // `"11:…Z"` (11:00Z) but is the EARLIER instant. Instant-based recency
        // must put the `Z` value (later) first; the old lexical compare failed this.
        let items = vec![
            row(
                "earlier",
                "Earlier",
                "/e.vdb",
                Some("2026-07-05T12:00:00.000+02:00"), // 10:00 UTC
                true,
            ),
            row(
                "later",
                "Later",
                "/l.vdb",
                Some("2026-07-05T11:00:00.000Z"), // 11:00 UTC
                true,
            ),
        ];
        assert_eq!(ids(&filter_sort_recents(&items, "")), ["later", "earlier"]);
    }

    #[test]
    fn name_breaks_ties() {
        // Same exists + both never opened → alphabetical by display name.
        let items = vec![
            row("bravo", "Bravo", "/b.vdb", None, true),
            row("alpha", "Alpha", "/a.vdb", None, true),
        ];
        assert_eq!(ids(&filter_sort_recents(&items, "")), ["alpha", "bravo"]);
    }
}
