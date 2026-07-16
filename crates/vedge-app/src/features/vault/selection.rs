//! Pure logic for multi-entry **selection** on the vault page (slice 5.3.1c).
//!
//! The two data-loss guards live here as pure, host-testable functions so they
//! are locked *before* any UI wiring (spec test plan #1/#2):
//!
//! - 🔴 **④ "Select all" is filtered-scoped.** The rows the header checkbox
//!   selects are exactly [`vault_filters::filter_and_sort`]'s output — never the
//!   whole vault. [`visible_ids`] names that set. *(Filter to 12, select-all →
//!   12, not 247. A filter-then-select-all-then-Trash must not wipe the vault.)*
//! - 🔴 **⑤ A filter/search change clears the selection; a sort change does
//!   not.** [`ViewIdentity`] captures the facets + the active/trashed source but
//!   **deliberately omits `SortKey`**, so a re-sort cannot change it. The page
//!   drives a prev-guarded `Effect` off a `Memo<ViewIdentity>` and clears the
//!   selection only when [`identity_changed`] reports a real difference.
//!
//! Selection is keyed by the entry's stable ULID throughout (⓪); these helpers
//! speak `String` ids, matching the `DataTable` selection API and `IndexEntryDto.id`.

use crate::features::vault::vault_filters::Filters;

/// The inputs that define **which rows are visible** — the facets plus the
/// active-vs-trashed source. A change to any of these hides or reveals rows, so
/// a surviving selection could act on entries the user can no longer see (⑤).
///
/// 🔴 `SortKey` is **not** a field: a re-sort shows the *same* set in a new
/// order, so it must not clear the selection. Leaving it out makes that
/// guarantee structural — the page's `Memo<ViewIdentity>` never reads `sort`, so
/// the clear `Effect` can't even re-run on a sort change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewIdentity {
    pub filters: Filters,
    /// The active (`false`) vs. trashed (`true`) fetch source. Not a facet in
    /// [`Filters`] (it switches the query upstream), but switching it swaps the
    /// whole visible set, so it belongs to the identity.
    pub trashed_view: bool,
}

impl ViewIdentity {
    #[must_use]
    pub fn new(filters: Filters, trashed_view: bool) -> Self {
        Self {
            filters,
            trashed_view,
        }
    }
}

/// Whether the visible set changed between two renders — i.e. whether the
/// selection must be cleared (⑤). A pure `!=`; the value is that the call site
/// reads as intent, and the tests below pin the semantics (sort is excluded, so
/// a sort-only change is never a change here).
#[must_use]
pub fn identity_changed(prev: &ViewIdentity, next: &ViewIdentity) -> bool {
    prev != next
}

/// Add every id in `add` that isn't already in `existing`, order-stable — the
/// union used by **bulk Tag** (⑥). `set_tags` *replaces* an entry's whole list,
/// so a bulk "add tag X" must union with each entry's current `tag_ids` first.
/// Add-only by design: removing a tag across a heterogeneous selection is
/// ambiguous, so this never drops an id.
#[must_use]
pub fn union_tags(existing: &[String], add: &[String]) -> Vec<String> {
    let mut out = existing.to_vec();
    for id in add {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vault::vault_filters::{Filters, SortKey, filter_and_sort};
    use std::collections::HashMap;
    use vedge_ipc::{EntryTypeDto, IndexEntryDto};

    /// A minimal Login entry (mirrors `vault_filters`' test fixture).
    fn entry(id: &str, name: &str) -> IndexEntryDto {
        IndexEntryDto {
            id: id.into(),
            name: name.into(),
            entry_type: EntryTypeDto::Login,
            url: None,
            favicon_url: None,
            tag_ids: vec![],
            folder_id: None,
            is_favorite: false,
            color: None,
            icon: None,
            sort_order: 0,
            is_trashed: false,
            cipher_suite: 1,
            created_at: "2026-07-05T00:00:00.000Z".into(),
            updated_at: "2026-07-05T00:00:00.000Z".into(),
            accessed_at: None,
        }
    }

    // ---- ④ select-all is FILTERED-scoped, never the whole vault --------------

    /// What the header checkbox / Ctrl+A select — **exactly the visible rows**.
    /// Mirrors `DataTable::select_all` over the keys it is handed; `visible` is
    /// already `filter_and_sort`'s output, so this cannot exceed the filter.
    fn visible_ids(visible: &[IndexEntryDto]) -> Vec<String> {
        visible.iter().map(|e| e.id.clone()).collect()
    }

    #[test]
    fn select_all_is_scoped_to_the_filter_not_the_vault() {
        // 247 entries; 12 are favorites.
        let mut items: Vec<IndexEntryDto> = (0..247)
            .map(|i| entry(&format!("id-{i}"), &format!("Entry {i}")))
            .collect();
        for it in items.iter_mut().take(12) {
            it.is_favorite = true;
        }

        // Filter to favorites → 12 visible.
        let f = Filters {
            favorites_only: true,
            ..Default::default()
        };
        let visible = filter_and_sort(&items, &f, &HashMap::new(), SortKey::NameAsc);
        assert_eq!(visible.len(), 12, "the filter yields 12 rows");

        // Select-all selects exactly those 12 — NOT 247. The data-loss guard.
        let selected = visible_ids(&visible);
        assert_eq!(
            selected.len(),
            12,
            "select-all is the filtered set, never the whole vault"
        );
        // And they are the favorite ids, not arbitrary vault ids.
        let fav_ids: Vec<String> = items
            .iter()
            .filter(|e| e.is_favorite)
            .map(|e| e.id.clone())
            .collect();
        let mut got = selected;
        let mut want = fav_ids;
        got.sort();
        want.sort();
        assert_eq!(
            got, want,
            "the selected ids are exactly the filtered entries"
        );
    }

    #[test]
    fn visible_ids_of_empty_is_empty() {
        assert!(visible_ids(&[]).is_empty());
    }

    // ---- ⑤ a filter/search change clears; a sort change does not -------------

    fn base_identity() -> ViewIdentity {
        ViewIdentity::new(Filters::default(), false)
    }

    #[test]
    fn a_query_change_clears_the_selection() {
        let prev = base_identity();
        let next = ViewIdentity::new(
            Filters {
                query: "github".into(),
                ..Default::default()
            },
            false,
        );
        assert!(identity_changed(&prev, &next));
    }

    #[test]
    fn a_facet_change_clears_the_selection() {
        let prev = base_identity();
        let type_change = ViewIdentity::new(
            Filters {
                entry_type: Some(EntryTypeDto::Card),
                ..Default::default()
            },
            false,
        );
        let tag_change = ViewIdentity::new(
            Filters {
                tag_id: Some("t-work".into()),
                ..Default::default()
            },
            false,
        );
        let fav_change = ViewIdentity::new(
            Filters {
                favorites_only: true,
                ..Default::default()
            },
            false,
        );
        assert!(identity_changed(&prev, &type_change), "type facet");
        assert!(identity_changed(&prev, &tag_change), "tag facet");
        assert!(identity_changed(&prev, &fav_change), "favorites facet");
    }

    #[test]
    fn a_scope_change_clears_the_selection() {
        use crate::features::vault::folder_tree::FolderScope;
        let prev = base_identity();
        let next = ViewIdentity::new(
            Filters {
                scope: FolderScope::Folder("f1".into()),
                ..Default::default()
            },
            false,
        );
        assert!(identity_changed(&prev, &next));
    }

    #[test]
    fn switching_to_the_trash_source_clears_the_selection() {
        let active = base_identity();
        let trashed = ViewIdentity::new(Filters::default(), true);
        assert!(
            identity_changed(&active, &trashed),
            "active↔trash swaps the whole visible set"
        );
    }

    #[test]
    fn sort_is_not_part_of_the_view_identity_so_a_re_sort_keeps_the_selection() {
        // The page builds the identity from filters + trashed_view only. Two
        // renders that differ *only* in SortKey therefore produce equal
        // identities — so `identity_changed` is false and the selection is kept
        // (⑤: same set, different order, nothing hidden). There is no `sort`
        // field to vary, which is the whole point.
        let a = base_identity();
        let b = base_identity();
        assert!(
            !identity_changed(&a, &b),
            "no filter/source change ⇒ selection survives (a sort-only change is exactly this case)"
        );
    }

    #[test]
    fn an_unchanged_identity_keeps_the_selection() {
        let f = Filters {
            query: "same".into(),
            favorites_only: true,
            ..Default::default()
        };
        let a = ViewIdentity::new(f.clone(), false);
        let b = ViewIdentity::new(f, false);
        assert!(!identity_changed(&a, &b));
    }

    // ---- bulk-tag union (⑥) --------------------------------------------------

    #[test]
    fn union_tags_appends_only_missing_ids() {
        let existing = vec!["a".to_owned(), "b".to_owned()];
        let add = vec!["b".to_owned(), "c".to_owned()];
        assert_eq!(
            union_tags(&existing, &add),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            "b is not duplicated; c is appended"
        );
    }

    #[test]
    fn union_tags_never_removes() {
        let existing = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(
            union_tags(&existing, &[]),
            existing,
            "add-only, never drops"
        );
    }
}
