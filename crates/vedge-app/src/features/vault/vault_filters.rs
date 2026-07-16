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
use crate::features::vault::tag_assign::toggle_tag;
use crate::features::vault::timestamps::ts_millis;
use crate::features::vault::vault_search::VaultSearch;
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashMap;
use vedge_ipc::{EntryTypeDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::feedback::popover::{Popover, PopoverPlacement};
use vedge_ui::components::foundation::badge::Badge;
use vedge_ui::components::segmented_control::{SegmentOption, SegmentedControl};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::components::toggle::Toggle;
use vedge_ui::components::{Button, IconButton};
use vedge_ui::primitives::tokens::{BadgeSize, BadgeVariant, Size, Variant};

/// Whether a multi-tag filter broadens (any) or narrows (all). Default `Any`
/// (OR): adding a tag shows *more*, never silently less. Persisted via
/// [`SmartFolder`](super::smart_folders::SmartFolder), so it carries serde.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagMatch {
    #[default]
    Any,
    All,
}

/// The active facet state. `query` matches name + url + tag names; the rest are
/// exact facets that AND together. Active-vs-trashed is handled by the source
/// fetch, not here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    pub query: String,
    pub entry_type: Option<EntryTypeDto>,
    /// Selected tag ids; empty = no tag facet. Combined by [`tag_match`](Self::tag_match).
    pub tag_ids: Vec<String>,
    /// How `tag_ids` combine (Any = OR, All = AND). Ignored when `tag_ids` is empty.
    pub tag_match: TagMatch,
    pub favorites_only: bool,
    /// Folder navigation scope. `All` (the default) shows everything; `Unfiled`
    /// shows entries with no folder; `Folder(id)` shows that folder's contents.
    pub scope: FolderScope,
}

/// Sort direction for a column-backed sort (Name / Url / Updated).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

/// How the filtered list is ordered. The column sorts carry a direction; the two
/// *modes* (`RecentlyUsed`, `Custom`) have no column and no direction — they are a
/// lens, not a header state (a smart folder can't hold `Custom`; see [`SavedSort`]).
/// Transient view state — **not persisted** (only [`SavedSort`] is).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Name, case-insensitive.
    Name(SortDir),
    /// Url, case-insensitive; a missing url sorts as empty.
    Url(SortDir),
    /// `updated_at` as an instant (Desc = most recently edited first).
    Updated(SortDir),
    /// `accessed_at` descending, entries never used (`None`) sorted last.
    RecentlyUsed,
    /// Manual drag order: `sort_order` ascending, ties by name. Only meaningful in
    /// a single folder scope, where a drag renumbers the siblings.
    Custom,
}

impl Default for SortKey {
    fn default() -> Self {
        SortKey::Name(SortDir::Asc)
    }
}

/// The subset of [`SortKey`] a `SmartFolder` may hold — **no `Custom`** (⑪: you
/// can arrange the icons in a folder, but not a search result; a preset can span
/// folders, whose per-folder `sort_order` interleaves meaninglessly). Making it a
/// narrower type turns "don't save Custom" from a rule into a compile error.
///
/// Serialized as a stable `snake_case` token via `String`, and it **reads the legacy
/// fieldless [`SortKey`] tokens** (`"NameAsc"`, `"RecentlyUpdated"`, `"RecentlyUsed"`,
/// `"Manual"`) so presets saved before this slice still load — `"Manual"` (the old
/// custom order) coerces to `Name(Asc)`. Unknown → `Name(Asc)`; never fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum SavedSort {
    Name(SortDir),
    Url(SortDir),
    Updated(SortDir),
    RecentlyUsed,
}

impl Default for SavedSort {
    fn default() -> Self {
        SavedSort::Name(SortDir::Asc)
    }
}

impl From<SavedSort> for String {
    fn from(s: SavedSort) -> String {
        match s {
            SavedSort::Name(SortDir::Asc) => "name_asc",
            SavedSort::Name(SortDir::Desc) => "name_desc",
            SavedSort::Url(SortDir::Asc) => "url_asc",
            SavedSort::Url(SortDir::Desc) => "url_desc",
            SavedSort::Updated(SortDir::Asc) => "updated_asc",
            SavedSort::Updated(SortDir::Desc) => "updated_desc",
            SavedSort::RecentlyUsed => "recently_used",
        }
        .to_owned()
    }
}

impl From<String> for SavedSort {
    fn from(s: String) -> SavedSort {
        match s.as_str() {
            "name_desc" => SavedSort::Name(SortDir::Desc),
            "url_asc" => SavedSort::Url(SortDir::Asc),
            "url_desc" => SavedSort::Url(SortDir::Desc),
            "updated_asc" => SavedSort::Updated(SortDir::Asc),
            "updated_desc" | "RecentlyUpdated" => SavedSort::Updated(SortDir::Desc),
            "recently_used" | "RecentlyUsed" => SavedSort::RecentlyUsed,
            // `Name(Asc)` is the default, so it is the fallback: the canonical
            // `"name_asc"`, the legacy `"NameAsc"`, the legacy custom `"Manual"` (a
            // preset can't be Custom — ⑪), and anything unknown all resolve here.
            // We never fail a read.
            _ => SavedSort::Name(SortDir::Asc),
        }
    }
}

impl From<SavedSort> for SortKey {
    fn from(s: SavedSort) -> SortKey {
        match s {
            SavedSort::Name(d) => SortKey::Name(d),
            SavedSort::Url(d) => SortKey::Url(d),
            SavedSort::Updated(d) => SortKey::Updated(d),
            SavedSort::RecentlyUsed => SortKey::RecentlyUsed,
        }
    }
}

/// Narrow a live [`SortKey`] to a [`SavedSort`] for persistence; `Custom` coerces
/// to `Name(Asc)` (it cannot be saved — ⑪).
#[must_use]
pub fn saved_from(sk: SortKey) -> SavedSort {
    match sk {
        SortKey::Name(d) => SavedSort::Name(d),
        SortKey::Url(d) => SavedSort::Url(d),
        SortKey::Updated(d) => SavedSort::Updated(d),
        SortKey::RecentlyUsed => SavedSort::RecentlyUsed,
        SortKey::Custom => SavedSort::Name(SortDir::Asc),
    }
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
        // Multi-tag: empty ⇒ no facet; else Any = OR, All = AND over the selected ids.
        .filter(|e| {
            f.tag_ids.is_empty()
                || match f.tag_match {
                    TagMatch::Any => f.tag_ids.iter().any(|id| e.tag_ids.contains(id)),
                    TagMatch::All => f.tag_ids.iter().all(|id| e.tag_ids.contains(id)),
                }
        })
        .filter(|e| !f.favorites_only || e.is_favorite)
        .cloned()
        .collect();

    // Timestamps are RFC-3339 DTO *strings*, so they are parsed to the absolute
    // instant before comparison — never compared lexically (a variable-width or
    // non-`Z` form would sort by text, not by time). `sort_by_cached_key` parses
    // once per element (O(n)) and is a stable sort, so equal instants keep input
    // order. See [`super::timestamps`] for the standing rule.
    match sort {
        // Column sorts lowercase/parse once per element (O(n)) and are stable —
        // equal keys keep input order. `Desc` sorts the `Reverse`d key (a stable
        // reverse, so it does not flip the tie order the way `out.reverse()` would).
        SortKey::Name(dir) => dir_key(&mut out, dir, |e| e.name.to_lowercase()),
        SortKey::Url(dir) => {
            dir_key(&mut out, dir, |e| {
                e.url.as_deref().unwrap_or_default().to_lowercase()
            });
        }
        SortKey::Updated(dir) => dir_key(&mut out, dir, |e| ts_millis(&e.updated_at)),
        SortKey::RecentlyUsed => out.sort_by_cached_key(|e| {
            // `None`/unparseable → `i64::MIN` → `Reverse` largest → sorts last.
            Reverse(e.accessed_at.as_deref().map_or(i64::MIN, ts_millis))
        }),
        // A `(u32, String)` tuple key is `Ord` (sort_order asc, ties by name).
        SortKey::Custom => out.sort_by_cached_key(|e| (e.sort_order, e.name.to_lowercase())),
    }
    out
}

/// Stable sort `out` by `key`, ascending or (for `Desc`) by the `Reverse`d key.
fn dir_key<K, F>(out: &mut [IndexEntryDto], dir: SortDir, key: F)
where
    K: Ord,
    F: Fn(&IndexEntryDto) -> K,
{
    match dir {
        SortDir::Asc => out.sort_by_cached_key(|e| key(e)),
        SortDir::Desc => out.sort_by_cached_key(|e| Reverse(key(e))),
    }
}

/// Recompute manual `sort_order` after dropping `moved_id` onto `target_id`
/// within `current` (the visible sibling order, each `(id, sort_order)`). The
/// dragged row lands immediately **before** the target; the whole run is then
/// renumbered `0..N`. Returns only the `(id, new_order)` pairs that actually
/// changed, so the caller persists the minimum. A no-op (same id, missing id, or
/// order unchanged) yields an empty vec.
#[must_use]
pub fn reorder_within(
    current: &[(String, u32)],
    moved_id: &str,
    target_id: &str,
) -> Vec<(String, u32)> {
    if moved_id == target_id {
        return Vec::new();
    }
    let mut ids: Vec<&str> = current.iter().map(|(id, _)| id.as_str()).collect();
    let Some(from) = ids.iter().position(|id| *id == moved_id) else {
        return Vec::new();
    };
    ids.remove(from);
    let Some(target_idx) = ids.iter().position(|id| *id == target_id) else {
        return Vec::new();
    };
    ids.insert(target_idx, moved_id);

    // Diff the new sequential order against each entry's current sort_order.
    let old: HashMap<&str, u32> = current.iter().map(|(id, o)| (id.as_str(), *o)).collect();
    ids.iter()
        .enumerate()
        .filter_map(|(idx, id)| {
            let new_order = u32::try_from(idx).unwrap_or(u32::MAX);
            (old.get(id).copied() != Some(new_order)).then(|| ((*id).to_owned(), new_order))
        })
        .collect()
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
    // The transitional toolbar Select can't express a direction; it maps each
    // column to its default. (d2 moves sort into the headers, which do carry a
    // direction, and drops this Select.)
    match s {
        SortKey::Name(_) => "name",
        SortKey::Url(_) => "url",
        SortKey::Updated(_) => "updated",
        SortKey::RecentlyUsed => "used",
        SortKey::Custom => "manual",
    }
}

/// Inverse of [`sort_to_key`]; unrecognized keys fall back to `Name(Asc)`.
fn sort_from_key(k: &str) -> SortKey {
    match k {
        "url" => SortKey::Url(SortDir::Asc),
        "updated" => SortKey::Updated(SortDir::Desc),
        "used" => SortKey::RecentlyUsed,
        "manual" => SortKey::Custom,
        _ => SortKey::Name(SortDir::Asc),
    }
}

/// The filter toolbar: a prominent search box, a **Filter** button (with an
/// active-facet count `Badge`) that opens a popover of facets — type, multi-select
/// tag pills with an any/all toggle, and favourites — plus the sort `Select` (which
/// moves into the table headers in d2b). Active facets show as removable chips via
/// [`FilterChips`], mounted separately by the page.
#[component]
pub fn VaultFilters(
    search_query: RwSignal<String>,
    entry_type: RwSignal<Option<EntryTypeDto>>,
    tag_ids: RwSignal<Vec<String>>,
    tag_match: RwSignal<TagMatch>,
    favorites_only: RwSignal<bool>,
    sort: RwSignal<SortKey>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let popover_open = RwSignal::new(false);
    let trigger_ref = NodeRef::<leptos::html::Div>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });
    let close = Callback::new(move |()| popover_open.set(false));

    // How many facets are active — drives the button's count badge.
    let active_count = move || {
        usize::from(entry_type.get().is_some())
            + usize::from(!tag_ids.with(Vec::is_empty))
            + usize::from(favorites_only.get())
    };

    view! {
        <div class="flex-1 flex items-center gap-2 min-w-0">
            <VaultSearch search_query=search_query />

            // Filter button — opens the facet popover, badged with the active count.
            <div node_ref=trigger_ref class="shrink-0">
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    class="whitespace-nowrap gap-1.5"
                    attr:data-testid="vault-filter"
                    on:click=move |_: web_sys::MouseEvent| popover_open.update(|v| *v = !*v)
                >
                    <Icon attr:aria-hidden="true" icon=i::FaFilterSolid />
                    {move || t!(i18n, vault.filter_button)}
                    {move || {
                        let n = active_count();
                        (n > 0)
                            .then(|| {
                                view! {
                                    <Badge variant=BadgeVariant::Info size=BadgeSize::Sm>
                                        {n.to_string()}
                                    </Badge>
                                }
                            })
                    }}
                </Button>
            </div>

            <Popover
                open=Signal::derive(move || popover_open.get())
                on_close=close
                anchor=anchor
                placement=PopoverPlacement::BottomStart
            >
                <div
                    class="flex flex-col gap-4 p-4 w-72"
                    role="group"
                    aria-label=move || t_string!(i18n, vault.filter_popover_aria).to_owned()
                >
                    // Type facet.
                    <div class="flex flex-col gap-1">
                        <span class="text-foreground/50 text-xs uppercase tracking-wider">
                            {move || t!(i18n, vault.filter_type)}
                        </span>
                        <Select
                            options=Signal::derive(move || {
                                let mut options = vec![
                                    SelectItem::option(
                                        "",
                                        t_string!(i18n, vault.filter_all_types).to_owned(),
                                    ),
                                ];
                                options
                                    .extend(
                                        filterable_types()
                                            .iter()
                                            .map(|ty| SelectItem::option(
                                                type_to_key(ty),
                                                type_label_i18n(i18n, ty),
                                            )),
                                    );
                                options
                            })
                            value=Signal::derive(move || {
                                entry_type.get().as_ref().map_or("", type_to_key).to_owned()
                            })
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, vault.filter_type).to_owned()
                            })
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, vault.filter_type_aria).to_owned()
                            })
                            on_change=Callback::new(move |key: String| {
                                entry_type
                                    .set(
                                        if key.is_empty() {
                                            None
                                        } else {
                                            Some(type_from_key(&key))
                                        },
                                    );
                            })
                        />
                    </div>

                    // Tags facet — multi-select pills + an any/all toggle (shown once
                    // two or more are picked, when the combine mode actually matters).
                    <div class="flex flex-col gap-2">
                        <div class="flex items-center justify-between gap-2 min-h-6">
                            <span class="text-foreground/50 text-xs uppercase tracking-wider">
                                {move || t!(i18n, vault.filter_tags_heading)}
                            </span>
                            {move || {
                                (tag_ids.with(|v| v.len() >= 2))
                                    .then(|| {
                                        view! {
                                            <div class="flex items-center gap-1.5">
                                                <span class="text-foreground/50 text-xs">
                                                    {move || t!(i18n, vault.tag_match_label)}
                                                </span>
                                                <SegmentedControl
                                                    options=vec![
                                                        SegmentOption::text(
                                                            "any",
                                                            Signal::derive(move || {
                                                                t_string!(i18n, vault.tag_match_any).to_owned()
                                                            }),
                                                        ),
                                                        SegmentOption::text(
                                                            "all",
                                                            Signal::derive(move || {
                                                                t_string!(i18n, vault.tag_match_all).to_owned()
                                                            }),
                                                        ),
                                                    ]
                                                    value=Signal::derive(move || {
                                                        match tag_match.get() {
                                                            TagMatch::Any => "any",
                                                            TagMatch::All => "all",
                                                        }
                                                            .to_owned()
                                                    })
                                                    size=Size::Sm
                                                    on_change=Callback::new(move |v: String| {
                                                        tag_match
                                                            .set(
                                                                if v == "all" { TagMatch::All } else { TagMatch::Any },
                                                            );
                                                    })
                                                    aria_label=Signal::derive(move || {
                                                        t_string!(i18n, vault.tag_match_aria).to_owned()
                                                    })
                                                />
                                            </div>
                                        }
                                    })
                            }}
                        </div>
                        <Show
                            when=move || !tags.get().is_empty()
                            fallback=move || {
                                view! {
                                    <span class="text-foreground/40 text-xs">
                                        {move || t!(i18n, vault.filter_all_tags)}
                                    </span>
                                }
                            }
                        >
                            <div class="flex flex-wrap gap-1.5">
                                {move || {
                                    tags.get()
                                        .into_iter()
                                        .map(|tag| {
                                            let id = tag.id;
                                            let name = tag.name;
                                            let cls_id = id.clone();
                                            let aria_id = id.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class=move || {
                                                        if tag_ids.with(|v| v.contains(&cls_id)) {
                                                            "inline-flex items-center rounded-full border px-2.5 py-1 text-xs transition-colors border-primary bg-primary/10 text-primary"
                                                        } else {
                                                            "inline-flex items-center rounded-full border px-2.5 py-1 text-xs transition-colors border-border text-foreground/70 hover:border-primary/60"
                                                        }
                                                    }
                                                    aria-pressed=move || {
                                                        tag_ids.with(|v| v.contains(&aria_id)).then_some("true")
                                                    }
                                                    on:click=move |_: web_sys::MouseEvent| {
                                                        tag_ids.update(|v| *v = toggle_tag(v, &id));
                                                    }
                                                >
                                                    {name}
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </Show>
                    </div>

                    // Favourites-only.
                    <div class="flex items-center gap-2 text-sm text-foreground/70">
                        <Toggle
                            checked=Signal::derive(move || favorites_only.get())
                            on_change=Callback::new(move |v: bool| favorites_only.set(v))
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, vault.filter_favorites).to_owned()
                            })
                        />
                        <span>{move || t!(i18n, vault.filter_favorites)}</span>
                    </div>
                </div>
            </Popover>

            // Sort key — transitional Select; moves into the table headers in d2b.
            <div class="w-40 shrink-0">
                <Select
                    options=Signal::derive(move || {
                        vec![
                            SelectItem::option("name", t_string!(i18n, vault.sort_name).to_owned()),
                            SelectItem::option(
                                "updated",
                                t_string!(i18n, vault.sort_updated).to_owned(),
                            ),
                            SelectItem::option("used", t_string!(i18n, vault.sort_used).to_owned()),
                            SelectItem::option(
                                "manual",
                                t_string!(i18n, vault.sort_manual).to_owned(),
                            ),
                        ]
                    })
                    value=Signal::derive(move || sort_to_key(sort.get()).to_owned())
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, vault.sort_label).to_owned()
                    })
                    aria_label=Signal::derive(move || {
                        t_string!(i18n, vault.sort_aria).to_owned()
                    })
                    on_change=Callback::new(move |v: String| sort.set(sort_from_key(&v)))
                />
            </div>
        </div>
    }
}

/// One removable filter chip: a label + an ✕ that clears the facet.
#[component]
fn FacetChip(text: String, remove_label: String, on_remove: Callback<()>) -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-1 rounded-full bg-primary/10 text-primary px-2 py-0.5">
            <span class="truncate max-w-[16rem]">{text}</span>
            <IconButton
                variant=Variant::Ghost
                size=Size::Xs
                class="hover:text-danger"
                aria_label=remove_label
                on:click=move |_: web_sys::MouseEvent| on_remove.run(())
            >
                <Icon attr:aria-hidden="true" icon=i::FaXmarkSolid />
            </IconButton>
        </span>
    }
}

/// Active-facet chips + **Clear all** + the result count (`12 of 247 entries`).
/// Each chip removes only its facet; Clear all resets the facets but **not** the
/// search. The Tags chip states the any/all semantics (`work or personal`), never
/// a bare count (⑦). Mounted by the page as a row under the toolbar.
#[component]
pub fn FilterChips(
    entry_type: RwSignal<Option<EntryTypeDto>>,
    tag_ids: RwSignal<Vec<String>>,
    tag_match: RwSignal<TagMatch>,
    favorites_only: RwSignal<bool>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
    /// Rows currently shown (after filtering).
    #[prop(into)]
    shown: Signal<usize>,
    /// Total listable rows in the current source (the denominator).
    #[prop(into)]
    total: Signal<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let any_active =
        move || entry_type.get().is_some() || !tag_ids.with(Vec::is_empty) || favorites_only.get();

    // Resolve the selected tag ids to names, joined by the any/all connector, so
    // the chip reads "work or personal" — the semantics, not a count.
    let tags_chip_text = move || {
        let selected = tag_ids.get();
        let names: Vec<String> = selected
            .iter()
            .filter_map(|id| {
                tags.with(|cat| cat.iter().find(|t| &t.id == id).map(|t| t.name.clone()))
            })
            .collect();
        let conj = if matches!(tag_match.get(), TagMatch::All) {
            t_string!(i18n, vault.filter_conj_and).to_owned()
        } else {
            t_string!(i18n, vault.filter_conj_or).to_owned()
        };
        names.join(&format!(" {conj} "))
    };

    view! {
        <div class="flex items-center flex-wrap gap-2 text-xs" data-testid="filter-chips">
            {move || {
                entry_type
                    .get()
                    .map(|ty| {
                        let label = type_label_i18n(i18n, &ty);
                        let remove = t_string!(i18n, vault.chip_remove).to_owned();
                        view! {
                            <FacetChip
                                text=label
                                remove_label=remove
                                on_remove=Callback::new(move |()| entry_type.set(None))
                            />
                        }
                    })
            }}
            {move || {
                (!tag_ids.with(Vec::is_empty))
                    .then(|| {
                        let remove = t_string!(i18n, vault.chip_remove).to_owned();
                        view! {
                            <FacetChip
                                text=tags_chip_text()
                                remove_label=remove
                                on_remove=Callback::new(move |()| tag_ids.set(Vec::new()))
                            />
                        }
                    })
            }}
            {move || {
                favorites_only
                    .get()
                    .then(|| {
                        let label = t_string!(i18n, vault.chip_favorites).to_owned();
                        let remove = t_string!(i18n, vault.chip_remove).to_owned();
                        view! {
                            <FacetChip
                                text=label
                                remove_label=remove
                                on_remove=Callback::new(move |()| favorites_only.set(false))
                            />
                        }
                    })
            }}
            <Show when=any_active>
                <button
                    type="button"
                    class="text-foreground/60 hover:text-foreground underline underline-offset-2"
                    data-testid="filter-clear-all"
                    on:click=move |_: web_sys::MouseEvent| {
                        entry_type.set(None);
                        tag_ids.set(Vec::new());
                        favorites_only.set(false);
                    }
                >
                    {move || t!(i18n, vault.filter_clear_all)}
                </button>
            </Show>
            <span class="ml-auto text-foreground/50 whitespace-nowrap">
                {move || {
                    let (shown, total) = (shown.get(), total.get());
                    t!(i18n, vault.filter_result_count, shown = shown, total = total)
                }}
            </span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Filters, SavedSort, SortDir, SortKey, TagMatch, filter_and_sort, reorder_within, saved_from,
    };
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
        let tag_names = HashMap::from([("t-work".to_owned(), "Work".to_owned())]);

        let q = |s: &str| Filters {
            query: s.into(),
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("hub"),
                &tag_names,
                SortKey::Name(SortDir::Asc)
            )),
            ["1"]
        );
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("example"),
                &tag_names,
                SortKey::Name(SortDir::Asc)
            )),
            ["2"]
        );
        // Matches *only* via the resolved tag name.
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &q("work"),
                &tag_names,
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
            )),
            ["c"]
        );
    }

    #[test]
    fn filter_by_tag() {
        // A single-element `tag_ids` behaves like the old single-tag facet.
        let mut a = entry("a", "A");
        a.tag_ids = vec!["t1".into(), "t2".into()];
        let mut b = entry("b", "B");
        b.tag_ids = vec!["t2".into()];
        let items = vec![a, b];
        let f = Filters {
            tag_ids: vec!["t1".into()],
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::Name(SortDir::Asc)
            )),
            ["a"]
        );
    }

    #[test]
    fn filter_by_tags_any_broadens() {
        // Any (OR): an entry with *either* tag matches.
        let mut a = entry("a", "A");
        a.tag_ids = vec!["work".into()];
        let mut b = entry("b", "B");
        b.tag_ids = vec!["personal".into()];
        let c = entry("c", "C"); // no tags
        let items = vec![a, b, c];
        let f = Filters {
            tag_ids: vec!["work".into(), "personal".into()],
            tag_match: TagMatch::Any,
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::default()
            )),
            ["a", "b"]
        );
    }

    #[test]
    fn filter_by_tags_all_narrows() {
        // All (AND): only an entry with *both* tags matches.
        let mut a = entry("a", "A");
        a.tag_ids = vec!["work".into(), "personal".into()];
        let mut b = entry("b", "B");
        b.tag_ids = vec!["work".into()];
        let items = vec![a, b];
        let f = Filters {
            tag_ids: vec!["work".into(), "personal".into()],
            tag_match: TagMatch::All,
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::default()
            )),
            ["a"]
        );
    }

    #[test]
    fn empty_tag_ids_is_no_tag_facet() {
        let mut a = entry("a", "A");
        a.tag_ids = vec!["work".into()];
        let items = vec![a, entry("b", "B")];
        let f = Filters {
            tag_ids: vec![],
            tag_match: TagMatch::All, // ignored when empty
            ..Default::default()
        };
        assert_eq!(
            ids(&filter_and_sort(
                &items,
                &f,
                &HashMap::new(),
                SortKey::default()
            )),
            ["a", "b"]
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
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
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
                SortKey::Name(SortDir::Asc)
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
            SortKey::Name(SortDir::Asc),
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
            SortKey::Updated(SortDir::Desc),
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
    fn sort_recently_updated_orders_by_instant_not_text() {
        // Regression guard for the old lexical compare: `"12:…+02:00"` (10:00Z)
        // is textually greater than `"11:…Z"` (11:00Z) but is the EARLIER instant.
        // A correct descending-recency sort puts the `Z` value (later) first — the
        // old `String::cmp` would have put "earlier" first and failed this test.
        let mut earlier = entry("earlier", "Earlier");
        earlier.updated_at = "2026-07-05T12:00:00.000+02:00".into(); // 10:00 UTC
        let mut later = entry("later", "Later");
        later.updated_at = "2026-07-05T11:00:00.000Z".into(); // 11:00 UTC
        let items = vec![earlier, later];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Updated(SortDir::Desc),
        );
        assert_eq!(ids(&r), ["later", "earlier"]);
    }

    #[test]
    fn sort_recently_used_orders_by_instant_not_text() {
        // Same lexical-vs-instant disagreement on `accessed_at`.
        let mut earlier = entry("earlier", "Earlier");
        earlier.accessed_at = Some("2026-07-05T12:00:00.000+02:00".into()); // 10:00 UTC
        let mut later = entry("later", "Later");
        later.accessed_at = Some("2026-07-05T11:00:00.000Z".into()); // 11:00 UTC
        let items = vec![earlier, later];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::RecentlyUsed,
        );
        assert_eq!(ids(&r), ["later", "earlier"]);
    }

    #[test]
    fn sort_recently_updated_unparseable_is_total_and_last() {
        // A garbage stamp maps to `i64::MIN` → sorts oldest (last), never panics.
        let mut good = entry("good", "Good");
        good.updated_at = "2026-01-01T00:00:00.000Z".into();
        let mut bad = entry("bad", "Bad");
        bad.updated_at = "not a timestamp".into();
        let items = vec![bad, good];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Updated(SortDir::Desc),
        );
        assert_eq!(ids(&r), ["good", "bad"]);
    }

    #[test]
    fn sort_manual_orders_by_sort_order_then_name() {
        let mut a = entry("a", "Zeta");
        a.sort_order = 0;
        let mut b = entry("b", "Alpha");
        b.sort_order = 2;
        let mut c = entry("c", "Mid");
        c.sort_order = 1;
        // Two entries share a sort_order → the tie breaks on name.
        let mut d = entry("d", "Beta");
        d.sort_order = 0;
        let items = vec![b, a, c, d];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Custom,
        );
        // order 0: Beta(d), Zeta(a) [name tie]; then Mid(c)=1; then Alpha(b)=2.
        assert_eq!(ids(&r), ["d", "a", "c", "b"]);
    }

    #[test]
    fn reorder_within_moves_row_and_diffs_changed_only() {
        // Current visible order a,b,c,d with sequential sort_order.
        let cur = vec![
            ("a".to_owned(), 0),
            ("b".to_owned(), 1),
            ("c".to_owned(), 2),
            ("d".to_owned(), 3),
        ];
        // Drag `d` onto `b` → d lands before b: a, d, b, c. Changed: d,b,c.
        let mut got = reorder_within(&cur, "d", "b");
        got.sort();
        assert_eq!(
            got,
            vec![
                ("b".to_owned(), 2),
                ("c".to_owned(), 3),
                ("d".to_owned(), 1),
            ]
        );
        // Drag `a` (top) onto `c` → a lands before c: b, a?  removal then insert
        // before c gives b, a, c, d → a=1, b=0. Changed: a,b.
        let mut down = reorder_within(&cur, "a", "c");
        down.sort();
        assert_eq!(down, vec![("a".to_owned(), 1), ("b".to_owned(), 0)]);
    }

    #[test]
    fn reorder_within_noops_are_empty() {
        let cur = vec![("a".to_owned(), 0), ("b".to_owned(), 1)];
        assert!(reorder_within(&cur, "a", "a").is_empty()); // same id
        assert!(reorder_within(&cur, "ghost", "a").is_empty()); // missing moved
        assert!(reorder_within(&cur, "a", "ghost").is_empty()); // missing target
    }

    #[test]
    fn empty_query_and_no_facets_returns_all_sorted() {
        let items = vec![entry("1", "Zeta"), entry("2", "Alpha")];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Name(SortDir::Asc),
        );
        assert_eq!(
            r.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "Zeta"]
        );
    }

    #[test]
    fn sort_name_desc_reverses() {
        let items = vec![
            entry("1", "banana"),
            entry("2", "Apple"),
            entry("3", "cherry"),
        ];
        let r = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Name(SortDir::Desc),
        );
        assert_eq!(
            r.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["cherry", "banana", "Apple"] // case-insensitive, reversed
        );
    }

    #[test]
    fn sort_by_url_asc_and_desc_missing_url_sorts_as_empty() {
        let mut a = entry("a", "A");
        a.url = Some("https://zebra.example".into());
        let mut b = entry("b", "B");
        b.url = Some("https://apple.example".into());
        let c = entry("c", "C"); // no url → sorts as ""
        let items = vec![a, b, c];
        // Asc: "" (c) first, then apple (b), then zebra (a).
        let asc = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Url(SortDir::Asc),
        );
        assert_eq!(ids(&asc), ["c", "b", "a"]);
        // Desc reverses.
        let desc = filter_and_sort(
            &items,
            &Filters::default(),
            &HashMap::new(),
            SortKey::Url(SortDir::Desc),
        );
        assert_eq!(ids(&desc), ["a", "b", "c"]);
    }

    #[test]
    fn sort_key_default_is_name_asc() {
        assert_eq!(SortKey::default(), SortKey::Name(SortDir::Asc));
    }

    #[test]
    fn saved_sort_round_trips_every_new_token() {
        for s in [
            SavedSort::Name(SortDir::Asc),
            SavedSort::Name(SortDir::Desc),
            SavedSort::Url(SortDir::Asc),
            SavedSort::Url(SortDir::Desc),
            SavedSort::Updated(SortDir::Asc),
            SavedSort::Updated(SortDir::Desc),
            SavedSort::RecentlyUsed,
        ] {
            let token: String = s.into();
            assert_eq!(SavedSort::from(token), s, "token round-trips");
        }
    }

    #[test]
    fn saved_sort_reads_legacy_tokens() {
        // The old fieldless `SortKey` serialized as these bare strings.
        assert_eq!(
            SavedSort::from("NameAsc".to_owned()),
            SavedSort::Name(SortDir::Asc)
        );
        assert_eq!(
            SavedSort::from("RecentlyUpdated".to_owned()),
            SavedSort::Updated(SortDir::Desc)
        );
        assert_eq!(
            SavedSort::from("RecentlyUsed".to_owned()),
            SavedSort::RecentlyUsed
        );
        // 🔴 The old custom order can't be saved (⑪) → coerces to Name(Asc).
        assert_eq!(
            SavedSort::from("Manual".to_owned()),
            SavedSort::Name(SortDir::Asc)
        );
        // Anything unrecognized also coerces, never fails.
        assert_eq!(
            SavedSort::from("garbage".to_owned()),
            SavedSort::Name(SortDir::Asc)
        );
    }

    #[test]
    fn saved_sort_survives_a_serde_json_round_trip() {
        let s = SavedSort::Updated(SortDir::Desc);
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"updated_desc\"");
        assert_eq!(serde_json::from_str::<SavedSort>(&json).unwrap(), s);
        // A legacy value deserializes too.
        assert_eq!(
            serde_json::from_str::<SavedSort>("\"Manual\"").unwrap(),
            SavedSort::Name(SortDir::Asc)
        );
    }

    #[test]
    fn saved_from_coerces_custom_and_widens_back() {
        // Custom has no SavedSort home → Name(Asc).
        assert_eq!(saved_from(SortKey::Custom), SavedSort::Name(SortDir::Asc));
        // Every SavedSort widens back to the same SortKey.
        for s in [
            SavedSort::Name(SortDir::Desc),
            SavedSort::Url(SortDir::Asc),
            SavedSort::Updated(SortDir::Desc),
            SavedSort::RecentlyUsed,
        ] {
            assert_eq!(saved_from(SortKey::from(s)), s);
        }
    }
}
