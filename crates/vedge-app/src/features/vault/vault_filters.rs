//! Client-side search / filter / sort over the loaded entry index.
//!
//! [`filter_and_sort`] is a pure, host-testable function (mirroring 2.1's
//! `EntryFormData` design): it filters the already-decrypted `IndexEntryDto`
//! metadata by a text query (matched over name + url + resolved **tag names**)
//! and the type / tag / favorites facets, then sorts. The active-vs-trashed
//! choice switches the *fetch source* upstream (in `vault.rs`), so it isn't a
//! facet here. [`VaultFilters`] is the thin control bar that binds the page's
//! signals and renders [`VaultSearch`] alongside the facet/sort dropdowns.

use crate::features::vault::entry_form::{type_from_key, type_to_key};
use crate::features::vault::entry_view::type_label_i18n;
use crate::features::vault::folder_tree::{FolderScope, scope_matches};
use crate::features::vault::vault_search::VaultSearch;
use crate::i18n::*;
use leptos::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use vedge_ipc::{EntryTypeDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::components::toggle::Toggle;

/// The active facet state. `query` matches name + url + tag names; the rest are
/// exact facets that AND together. Active-vs-trashed is handled by the source
/// fetch, not here.
#[derive(Clone, Default)]
pub struct Filters {
    pub query: String,
    pub entry_type: Option<EntryTypeDto>,
    pub tag_id: Option<String>,
    pub favorites_only: bool,
    /// Folder navigation scope. `All` (the default) shows everything; `Unfiled`
    /// shows entries with no folder; `Folder(id)` shows that folder's contents.
    pub scope: FolderScope,
}

/// How the filtered list is ordered.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum SortKey {
    /// Name, case-insensitive A–Z.
    #[default]
    NameAsc,
    /// `updated_at` descending (most recently edited first).
    RecentlyUpdated,
    /// `accessed_at` descending, entries never used (`None`) sorted last.
    RecentlyUsed,
}

/// Filter `items` by `f` (query + facets) and order by `sort`.
///
/// `tag_names` maps `tag_id -> name` (from `list_tags`) so the query can match a
/// tag's display name even though `IndexEntryDto` only carries `tag_ids`. Pure —
/// no signal reads — so it is unit-tested on the host.
#[must_use]
pub fn filter_and_sort(
    items: &[IndexEntryDto],
    f: &Filters,
    tag_names: &HashMap<String, String>,
    sort: SortKey,
) -> Vec<IndexEntryDto> {
    let q = f.query.trim().to_lowercase();
    let mut out: Vec<IndexEntryDto> = items
        .iter()
        // Folders are navigated in the tree, never listed as rows.
        .filter(|e| e.entry_type != EntryTypeDto::Folder)
        .filter(|e| query_matches(e, &q, tag_names))
        .filter(|e| scope_matches(&f.scope, e))
        .filter(|e| f.entry_type.as_ref().is_none_or(|t| &e.entry_type == t))
        .filter(|e| f.tag_id.as_ref().is_none_or(|id| e.tag_ids.contains(id)))
        .filter(|e| !f.favorites_only || e.is_favorite)
        .cloned()
        .collect();

    // RFC-3339 millis-precision UTC (`…Z`) strings are fixed-width, so a lexical
    // compare is chronological — no date parsing needed.
    match sort {
        SortKey::NameAsc => out.sort_by_key(|e| e.name.to_lowercase()),
        SortKey::RecentlyUpdated => out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
        SortKey::RecentlyUsed => {
            out.sort_by(|a, b| cmp_accessed_desc(&a.accessed_at, &b.accessed_at));
        }
    }
    out
}

/// Case-insensitive substring match over name, url, and each tag's resolved
/// name. `q` is expected pre-trimmed and lowercased; empty ⇒ matches all.
/// Reused by the command palette (2.3) for quick-open filtering.
pub fn query_matches(e: &IndexEntryDto, q: &str, tag_names: &HashMap<String, String>) -> bool {
    if q.is_empty() {
        return true;
    }
    if e.name.to_lowercase().contains(q) {
        return true;
    }
    if e.url
        .as_deref()
        .unwrap_or_default()
        .to_lowercase()
        .contains(q)
    {
        return true;
    }
    e.tag_ids
        .iter()
        .filter_map(|id| tag_names.get(id))
        .any(|name| name.to_lowercase().contains(q))
}

/// `accessed_at` descending with `None` sorted last.
fn cmp_accessed_desc(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// The entry types offered in the type facet — the listable variants (including
/// `Document`). `Folder` is navigated in the tree, not listed, so it's not
/// offered; the `Unknown` catch-all isn't either.
fn filterable_types() -> [EntryTypeDto; 8] {
    [
        EntryTypeDto::Login,
        EntryTypeDto::Card,
        EntryTypeDto::SshKey,
        EntryTypeDto::ApiKey,
        EntryTypeDto::EnvVars,
        EntryTypeDto::Note,
        EntryTypeDto::Document,
        EntryTypeDto::Identity,
    ]
}

/// Stable `Select` option value for a [`SortKey`].
fn sort_to_key(s: SortKey) -> &'static str {
    match s {
        SortKey::NameAsc => "name",
        SortKey::RecentlyUpdated => "updated",
        SortKey::RecentlyUsed => "used",
    }
}

/// Inverse of [`sort_to_key`]; unrecognized keys fall back to `NameAsc`.
fn sort_from_key(k: &str) -> SortKey {
    match k {
        "updated" => SortKey::RecentlyUpdated,
        "used" => SortKey::RecentlyUsed,
        _ => SortKey::NameAsc,
    }
}

/// The filter/sort toolbar: search box + type / tag / view / sort dropdowns +
/// a favorites toggle. Each control is bound to a page-owned `RwSignal`; the
/// derived list (and the fetch source, for `trashed_view`) reacts in `vault.rs`.
#[component]
pub fn VaultFilters(
    search_query: RwSignal<String>,
    entry_type: RwSignal<Option<EntryTypeDto>>,
    tag_id: RwSignal<Option<String>>,
    favorites_only: RwSignal<bool>,
    sort: RwSignal<SortKey>,
    trashed_view: RwSignal<bool>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex-1 flex flex-wrap items-center gap-2 min-w-0">
            <VaultSearch search_query=search_query />

            // Type facet. Option list built inside a reactive closure so the
            // locale read is tracked (no owner-less warning) and relocalizes.
            <div class="w-40 shrink-0">
                {move || {
                    let mut options = vec![SelectItem::option(
                        "",
                        t_string!(i18n, vault.filter_all_types).to_string(),
                    )];
                    options.extend(
                        filterable_types()
                            .iter()
                            .map(|ty| SelectItem::option(type_to_key(ty), type_label_i18n(i18n, ty))),
                    );
                    view! {
                        <Select
                            options=options
                            value=Signal::derive(move || {
                                entry_type.get().as_ref().map_or("", type_to_key).to_owned()
                            })
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, vault.filter_type).to_string()
                            })
                            on_change=Callback::new(move |key: String| {
                                entry_type
                                    .set(if key.is_empty() { None } else { Some(type_from_key(&key)) });
                            })
                        />
                    }
                }}
            </div>

            // Tag facet — rebuilt when the tag list or locale changes.
            <div class="w-40 shrink-0">
                {move || {
                    let mut options = vec![SelectItem::option(
                        "",
                        t_string!(i18n, vault.filter_all_tags).to_string(),
                    )];
                    options
                        .extend(tags.get().into_iter().map(|t| SelectItem::option(t.id, t.name)));
                    view! {
                        <Select
                            options=options
                            value=Signal::derive(move || tag_id.get().unwrap_or_default())
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, vault.filter_tag).to_string()
                            })
                            on_change=Callback::new(move |id: String| {
                                tag_id.set(if id.is_empty() { None } else { Some(id) });
                            })
                        />
                    }
                }}
            </div>

            // Active vs trashed — switches the fetch source in vault.rs.
            <div class="w-32 shrink-0">
                {move || {
                    let options = vec![
                        SelectItem::option("active", t_string!(i18n, vault.view_active).to_string()),
                        SelectItem::option("trash", t_string!(i18n, vault.view_trash).to_string()),
                    ];
                    view! {
                        <Select
                            options=options
                            value=Signal::derive(move || {
                                if trashed_view.get() { "trash" } else { "active" }.to_owned()
                            })
                            on_change=Callback::new(move |v: String| trashed_view.set(v == "trash"))
                        />
                    }
                }}
            </div>

            // Sort key.
            <div class="w-40 shrink-0">
                {move || {
                    let options = vec![
                        SelectItem::option("name", t_string!(i18n, vault.sort_name).to_string()),
                        SelectItem::option("updated", t_string!(i18n, vault.sort_updated).to_string()),
                        SelectItem::option("used", t_string!(i18n, vault.sort_used).to_string()),
                    ];
                    view! {
                        <Select
                            options=options
                            value=Signal::derive(move || sort_to_key(sort.get()).to_owned())
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, vault.sort_label).to_string()
                            })
                            on_change=Callback::new(move |v: String| sort.set(sort_from_key(&v)))
                        />
                    }
                }}
            </div>

            // Favorites-only. Toggle carries the aria-label; the visible text is
            // an adjacent span (nesting a <label> inside the Toggle's own label
            // would be invalid).
            <div class="flex items-center gap-2 text-sm text-foreground/70 whitespace-nowrap shrink-0">
                <Toggle
                    checked=Signal::derive(move || favorites_only.get())
                    on_change=Callback::new(move |v: bool| favorites_only.set(v))
                    aria_label=Signal::derive(move || t_string!(i18n, vault.filter_favorites).to_string())
                />
                <span>{move || t!(i18n, vault.filter_favorites)}</span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{Filters, SortKey, filter_and_sort};
    use std::collections::HashMap;
    use vedge_ipc::{EntryTypeDto, IndexEntryDto};

    /// A minimal Login entry; tests mutate the fields they exercise.
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
            is_trashed: false,
            cipher_suite: 1,
            created_at: "2026-07-05T00:00:00.000Z".into(),
            updated_at: "2026-07-05T00:00:00.000Z".into(),
            accessed_at: None,
        }
    }

    fn ids(v: &[IndexEntryDto]) -> Vec<&str> {
        v.iter().map(|e| e.id.as_str()).collect()
    }

    #[test]
    fn query_matches_name_url_and_tag_name() {
        let by_name = entry("1", "GitHub");
        let mut by_url = entry("2", "Thing");
        by_url.url = Some("https://example.com".into());
        let mut by_tag = entry("3", "Zzz"); // name/url hold no "work" text
        by_tag.tag_ids = vec!["t-work".into()];
        let items = vec![by_name, by_url, by_tag];
        let tag_names = HashMap::from([("t-work".to_string(), "Work".to_string())]);

        let q = |s: &str| Filters {
            query: s.into(),
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("hub"),
                &tag_names,
                SortKey::NameAsc
            )),
            ["1"]
        );
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("example"),
                &tag_names,
                SortKey::NameAsc
            )),
            ["2"]
        );
        // Matches *only* via the resolved tag name.
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("work"),
                &tag_names,
                SortKey::NameAsc
            )),
            ["3"]
        );
    }

    #[test]
    fn filter_by_type() {
        let mut card = entry("c", "Visa");
        card.entry_type = EntryTypeDto::Card;
        let items = vec![card, entry("l", "GitHub")];
        let f = Filters {
            entry_type: Some(EntryTypeDto::Card),
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["c"]
        );
    }

    #[test]
    fn filter_by_tag() {
        let mut a = entry("a", "A");
        a.tag_ids = vec!["t1".into(), "t2".into()];
        let mut b = entry("b", "B");
        b.tag_ids = vec!["t2".into()];
        let items = vec![a, b];
        let f = Filters {
            tag_id: Some("t1".into()),
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["a"]
        );
    }

    #[test]
    fn favorites_only() {
        let mut fav = entry("f", "Fav");
        fav.is_favorite = true;
        let items = vec![fav, entry("p", "Plain")];
        let f = Filters {
            favorites_only: true,
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["f"]
        );
    }

    #[test]
    fn folders_are_never_listed() {
        let mut folder = entry("f", "My Folder");
        folder.entry_type = EntryTypeDto::Folder;
        let items = vec![folder, entry("l", "GitHub")];
        // Even the default (no facets) view drops folder-type entries.
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &Filters::default(),
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["l"]
        );
    }

    #[test]
    fn filter_by_folder() {
        use crate::features::vault::folder_tree::FolderScope;
        let mut in_a = entry("a", "A");
        in_a.folder_id = Some("f1".into());
        let mut in_b = entry("b", "B");
        in_b.folder_id = Some("f2".into());
        let root = entry("r", "Root"); // no folder
        let items = vec![in_a, in_b, root];

        // `All` (default) passes everything through.
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &Filters::default(),
                &HashMap::new(),
                SortKey::NameAsc
            ))
            .len(),
            3
        );
        // `Folder(id)` narrows to that folder's direct members.
        let f = Filters {
            scope: FolderScope::Folder("f1".into()),
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["a"]
        );
        // `Unfiled` narrows to entries with no folder.
        let f = Filters {
            scope: FolderScope::Unfiled,
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["r"]
        );
    }

    #[test]
    fn facets_compose() {
        let mut a = entry("a", "A");
        a.entry_type = EntryTypeDto::Card;
        a.is_favorite = true;
        let mut b = entry("b", "B");
        b.entry_type = EntryTypeDto::Card; // not favorite
        let mut c = entry("c", "C");
        c.is_favorite = true; // not a Card
        let items = vec![a, b, c];
        let f = Filters {
            entry_type: Some(EntryTypeDto::Card),
            favorites_only: true,
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::NameAsc
            )),
            ["a"]
        );
    }

    #[test]
    fn sort_name_asc() {
        let items = vec![
            entry("1", "banana"),
            entry("2", "Apple"),
            entry("3", "cherry"),
        ];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::NameAsc,
        );
        assert_eq!(
            r.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["Apple", "banana", "cherry"] // case-insensitive
        );
    }

    #[test]
    fn sort_recently_updated() {
        let mut a = entry("a", "A");
        a.updated_at = "2026-01-01T00:00:00.000Z".into();
        let mut b = entry("b", "B");
        b.updated_at = "2026-03-01T00:00:00.000Z".into();
        let mut c = entry("c", "C");
        c.updated_at = "2026-02-01T00:00:00.000Z".into();
        let items = vec![a, b, c];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::RecentlyUpdated,
        );
        assert_eq!(ids(&r), ["b", "c", "a"]);
    }

    #[test]
    fn sort_recently_used_nulls_last() {
        let mut a = entry("a", "A");
        a.accessed_at = Some("2026-01-01T00:00:00.000Z".into());
        let mut b = entry("b", "B");
        b.accessed_at = Some("2026-03-01T00:00:00.000Z".into());
        let c = entry("c", "C"); // never accessed → None
        let items = vec![a, c, b]; // None deliberately in the middle
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::RecentlyUsed,
        );
        assert_eq!(ids(&r), ["b", "a", "c"]);
    }

    #[test]
    fn empty_query_and_no_facets_returns_all_sorted() {
        let items = vec![entry("1", "Zeta"), entry("2", "Alpha")];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::NameAsc,
        );
        assert_eq!(
            r.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "Zeta"]
        );
    }
}
