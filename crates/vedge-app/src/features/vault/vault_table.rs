//! The vault's entries rendered as a `DataTable`. Migrated from a hand-rolled
//! `<table>` (slice 5.3.1b) to clear the Gate 1.0 a11y blocker — `role="grid"`,
//! roving focus, `aria-selected`, keyboard nav all come from `DataTable`.
//!
//! Because a `DataTable` cell closure must be `Send + Sync` (so it cannot call
//! `t_string!` — that captures the non-`Send` `I18nContext` — nor read app
//! signals), each entry is pre-resolved into a [`VaultRow`] view-model in a
//! reactive `Signal::derive` *before* the table (the health/audit pattern). The
//! interactive cells (favourite + the action cluster) capture the page's
//! `Callback`s directly, which is sound: Leptos `Callback` is `Send + Sync`.

use super::entry_view::{short_date, type_icon, type_label_i18n};
use super::vault_filters::{SortDir, SortKey};
use crate::i18n::{Locale, t_string, use_i18n};
use leptos::either::Either;
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use std::collections::HashMap;
use vedge_ipc::{IndexEntryDto, TagMetaDto};
use vedge_ui::components::data_display::{
    CellValue, ColumnDef, ColumnType, ColumnWidth, DataTable, SortState, cell_fn, string_fn,
};
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Align, Size, SortDirection, Variant};

use icondata as i;
use leptos_icons::Icon;

/// A resolved entry row. Every display string is final (localized / formatted)
/// so the `Send + Sync` cell closures read plain data. `entry` is carried whole
/// because the row-click / move / hard-delete callbacks take the full DTO.
#[derive(Clone)]
struct VaultRow {
    id: String,
    entry: IndexEntryDto,
    name: String,
    url: String,
    type_label: String,
    /// Entry-type glyph shown in the Name cell (Type is no longer its own column).
    type_icon: icondata::Icon,
    updated: String,
    is_favorite: bool,
    favorite_aria: String,
    move_aria: String,
    delete_aria: String,
    restore_aria: String,
    hard_delete_aria: String,
    /// Per-row selection-checkbox aria-label ("Select {name}") — pre-resolved
    /// because the `DataTable` `row_select_label` closure is `Send + Sync` and so
    /// cannot call `t_string!`.
    select_aria: String,
    /// Resolved tag chips (name, optional color); unresolved ids are skipped.
    chips: Vec<(String, Option<String>)>,
    /// Whether this is the trashed view (Restore + Delete-permanent instead of
    /// Move + Delete). Carried per-row so the action cell reads plain data.
    hide_delete: bool,
}

fn build_row(
    i18n: I18nContext<Locale>,
    e: &IndexEntryDto,
    tags: &HashMap<String, TagMetaDto>,
    hide_delete: bool,
) -> VaultRow {
    let chips = e
        .tag_ids
        .iter()
        .filter_map(|id| tags.get(id).map(|m| (m.name.clone(), m.color.clone())))
        .collect();
    VaultRow {
        id: e.id.clone(),
        name: e.name.clone(),
        url: e.url.clone().unwrap_or_default(),
        type_label: type_label_i18n(i18n, &e.entry_type),
        type_icon: type_icon(&e.entry_type),
        updated: short_date(&e.updated_at),
        is_favorite: e.is_favorite,
        favorite_aria: if e.is_favorite {
            t_string!(i18n, vault.unfavorite).to_owned()
        } else {
            t_string!(i18n, vault.favorite).to_owned()
        },
        move_aria: t_string!(i18n, vault.folder_move).to_owned(),
        delete_aria: t_string!(i18n, vault.delete).to_owned(),
        restore_aria: t_string!(i18n, vault.restore).to_owned(),
        hard_delete_aria: t_string!(i18n, vault.delete_permanently).to_owned(),
        select_aria: format!("{} {}", t_string!(i18n, vault.row_select_aria), e.name),
        chips,
        hide_delete,
        entry: e.clone(),
    }
}

#[component]
pub fn VaultTable(
    #[prop(into)] items: Signal<Vec<IndexEntryDto>>,
    /// Message shown when `items` is empty — the page picks *no entries* vs
    /// *no matches* vs *empty trash* so this component stays presentational.
    #[prop(into)]
    empty_label: Signal<String>,
    /// Hide the per-row Delete action (the trashed view is read-only here).
    #[prop(into)]
    hide_delete: Signal<bool>,
    /// `tag_id -> TagMetaDto` for resolving each row's `tag_ids` to name + color.
    #[prop(into)]
    tags: Signal<HashMap<String, TagMetaDto>>,
    on_delete: Callback<String>,
    /// Restore a trashed entry to the active vault (trash view only). Takes the id.
    on_restore: Callback<String>,
    /// Permanently delete a trashed entry (trash view only) — opens the confirm. Takes the entry.
    on_hard_delete: Callback<IndexEntryDto>,
    on_select: Callback<IndexEntryDto>,
    /// `(entry_id, next_state)` — flip the row's favorite flag.
    on_favorite: Callback<(String, bool)>,
    /// Open the move-to-folder picker for this entry.
    on_move_request: Callback<IndexEntryDto>,
    /// `(moved_id, target_id)` — a drag-reorder dropped `moved` onto `target`.
    on_reorder: Callback<(String, String)>,
    /// Whether drag-reorder is active (only under the Manual sort).
    #[prop(into)]
    reorder_enabled: Signal<bool>,
    /// Whether the selection column is shown (5.3.1c). **Read once at mount** —
    /// the page remounts `VaultTable` on every active/trash switch (via its
    /// `loading` `<Show>`), and the value is constant within a mount, so a
    /// mount-time snapshot is always correct for the current view.
    #[prop(into, default = Signal::stored(false))]
    selectable: Signal<bool>,
    /// The selected entry ids (ULIDs), controlled by the page.
    #[prop(into, default = Signal::stored(Vec::<String>::new()))]
    selected_rows: Signal<Vec<String>>,
    /// Fires the new selection set (ULIDs) whenever the selection changes.
    #[prop(into, default = None)]
    on_selection_change: Option<Callback<Vec<String>>>,
    /// The active sort. Column sorts (Name/Url/Updated) drive the header
    /// `aria-sort`; the modes (RecentlyUsed/Custom) mark no header. Clicking a
    /// sortable header updates it (a 2-state Asc/Desc cycle).
    sort: RwSignal<SortKey>,
) -> impl IntoView {
    let i18n = use_i18n();
    // Mount-time snapshot (see the prop doc): `DataTable::selectable` is a static
    // bool, and the page guarantees a remount whenever the active/trash view flips.
    let is_selectable = selectable.get_untracked();

    // Pre-resolve rows. Reads items + tags + hide_delete + i18n, so it re-derives
    // on any of them — matching today's optimistic in-place `items` mutations and
    // relocalizing on a language switch.
    let rows = Signal::derive(move || {
        let tags = tags.get();
        let hide = hide_delete.get();
        items
            .get()
            .iter()
            .map(|e| build_row(i18n, e, &tags, hide))
            .collect::<Vec<_>>()
    });

    let columns = vec![
        // Favourite star — Custom so the whole cell isn't a stop-propagation zone
        // (only the button stops); clicking the cell padding still opens the row.
        ColumnDef {
            id: "favorite",
            header: "".into(),
            col_type: ColumnType::Custom,
            sortable: false,
            width: ColumnWidth::Fixed(44),
            align: Align::Start,
            cell: cell_fn(move |r: &VaultRow| {
                let id = r.id.clone();
                let is_fav = r.is_favorite;
                let aria = r.favorite_aria.clone();
                CellValue::View(
                    view! {
                        <IconButton
                            attr:data-testid="row-favorite"
                            aria_label=aria
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                on_favorite.run((id.clone(), !is_fav));
                            }
                        >
                            {if is_fav {
                                Either::Left(
                                    view! { <Icon attr:aria-hidden="true" icon=i::FaStarSolid /> },
                                )
                            } else {
                                Either::Right(
                                    view! { <Icon attr:aria-hidden="true" icon=i::FaStarRegular /> },
                                )
                            }}
                        </IconButton>
                    }
                    .into_any(),
                )
            }),
        },
        // Name — a type glyph (tooltip = type label; ⑨ frees the Type column) + the
        // name. Sortable via the header.
        ColumnDef {
            id: "name",
            header: Signal::derive(move || t_string!(i18n, vault.col_name).to_owned()).into(),
            col_type: ColumnType::Custom,
            sortable: true,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|r: &VaultRow| {
                let name = r.name.clone();
                let title = r.name.clone();
                let icon = r.type_icon;
                let type_title = r.type_label.clone();
                CellValue::View(
                    view! {
                        <div class="flex items-center gap-2 min-w-0">
                            <span class="shrink-0 text-foreground/40" title=type_title>
                                <Icon attr:aria-hidden="true" icon=icon />
                            </span>
                            <span class="block truncate" title=title>{name}</span>
                        </div>
                    }
                    .into_any(),
                )
            }),
        },
        // Tags — resolved chips. **Not sortable** (⑨): a multi-value field has no
        // natural order, and "first tag alphabetically" is a stable-looking lie.
        ColumnDef {
            id: "tags",
            header: Signal::derive(move || t_string!(i18n, vault.col_tags).to_owned()).into(),
            col_type: ColumnType::Custom,
            sortable: false,
            width: ColumnWidth::MinMax(120, 220),
            align: Align::Start,
            cell: cell_fn(|r: &VaultRow| {
                let chips = r.chips.clone();
                CellValue::View(
                    view! {
                        <div class="flex flex-wrap gap-1">
                            {chips
                                .into_iter()
                                .map(|(nm, color)| {
                                    let style = color
                                        .map(|c| format!("color:{c}"))
                                        .unwrap_or_default();
                                    view! {
                                        <span
                                            class="inline-flex items-center rounded-full bg-primary/10 text-primary text-[10px] px-1.5 py-0.5"
                                            style=style
                                        >
                                            {nm}
                                        </span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any(),
                )
            }),
        },
        // Url — mono, truncated, with a title tooltip for the full value. Sortable.
        ColumnDef {
            id: "url",
            header: Signal::derive(move || t_string!(i18n, vault.col_url).to_owned()).into(),
            col_type: ColumnType::Custom,
            sortable: true,
            width: ColumnWidth::MinMax(120, 240),
            align: Align::Start,
            cell: cell_fn(|r: &VaultRow| {
                let url = r.url.clone();
                let title = r.url.clone();
                CellValue::View(
                    view! {
                        <span
                            class="block truncate font-jetbrains-mono text-foreground/60"
                            title=title
                        >
                            {url}
                        </span>
                    }
                    .into_any(),
                )
            }),
        },
        // Updated — pre-resolved short date. Sortable.
        ColumnDef {
            id: "updated",
            header: Signal::derive(move || t_string!(i18n, vault.col_updated).to_owned()).into(),
            col_type: ColumnType::Date,
            sortable: true,
            width: ColumnWidth::Fixed(120),
            align: Align::End,
            cell: cell_fn(|r: &VaultRow| CellValue::Text(r.updated.clone())),
        },
        // Actions — Move + Delete (active) or Restore + Delete-permanent (trash).
        ColumnDef {
            id: "actions",
            header: Signal::derive(move || t_string!(i18n, vault.col_actions).to_owned()).into(),
            col_type: ColumnType::Action,
            sortable: false,
            width: ColumnWidth::Fixed(80),
            align: Align::End,
            cell: cell_fn(move |r: &VaultRow| {
                if r.hide_delete {
                    let restore_id = r.id.clone();
                    let entry = r.entry.clone();
                    let restore_aria = r.restore_aria.clone();
                    let hard_delete_aria = r.hard_delete_aria.clone();
                    CellValue::View(
                        view! {
                            <div class="flex items-center gap-1">
                                <IconButton
                                    attr:data-testid="row-restore"
                                    aria_label=restore_aria
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_restore.run(restore_id.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::FaTrashArrowUpSolid />
                                </IconButton>
                                <IconButton
                                    attr:data-testid="row-delete-permanent"
                                    aria_label=hard_delete_aria
                                    variant=Variant::Danger
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_hard_delete.run(entry.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::FaTrashCanSolid />
                                </IconButton>
                            </div>
                        }
                        .into_any(),
                    )
                } else {
                    let entry = r.entry.clone();
                    let del_id = r.id.clone();
                    let move_aria = r.move_aria.clone();
                    let delete_aria = r.delete_aria.clone();
                    CellValue::View(
                        view! {
                            <div class="flex items-center gap-1">
                                <IconButton
                                    attr:data-testid="row-move"
                                    aria_label=move_aria
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_move_request.run(entry.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::FaFolderOpenSolid />
                                </IconButton>
                                <IconButton
                                    attr:data-testid="row-delete"
                                    aria_label=delete_aria
                                    variant=Variant::Danger
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_delete.run(del_id.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::BiTrashRegular />
                                </IconButton>
                            </div>
                        }
                        .into_any(),
                    )
                }
            }),
        },
    ];

    view! {
        <div class="flex-1 min-w-0 overflow-auto">
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|r: &VaultRow| r.id.clone())
                row_testid=string_fn(|r: &VaultRow| r.id.clone())
                selectable=is_selectable
                selected_rows=selected_rows
                on_selection_change=on_selection_change
                select_all_label=Signal::derive(move || {
                    t_string!(i18n, vault.select_all_aria).to_owned()
                })
                deselect_all_label=Signal::derive(move || {
                    t_string!(i18n, vault.deselect_all_aria).to_owned()
                })
                row_select_label=string_fn(|r: &VaultRow| r.select_aria.clone())
                sort=Signal::derive(move || to_sort_state(sort.get()))
                on_sort_change=Callback::new(move |next: Option<SortState>| {
                    sort.set(from_sort_state(next, sort.get_untracked()));
                })
                on_row_click=Callback::new(move |r: VaultRow| on_select.run(r.entry))
                reorder_enabled=reorder_enabled
                on_row_reorder=on_reorder
                // Entries are always draggable onto the folder tree (which reads
                // the row key = entry id off `text/plain`) — restoring the
                // drag-to-folder the pre-DataTable table had in every sort. Only
                // Manual sort (`reorder_enabled`) makes the table a drop target.
                row_draggable=Signal::stored(true)
                empty_message=empty_label
            />
        </div>
    }
}

/// Map the page's [`SortKey`] to a `DataTable` header sort state. The two *modes*
/// (`RecentlyUsed`, `Custom`) have no sortable column, so **no header** is marked.
fn to_sort_state(sk: SortKey) -> Option<SortState> {
    let (column_id, dir) = match sk {
        SortKey::Name(d) => ("name", d),
        SortKey::Url(d) => ("url", d),
        SortKey::Updated(d) => ("updated", d),
        SortKey::RecentlyUsed | SortKey::Custom => return None,
    };
    Some(SortState {
        column_id: column_id.to_owned(),
        direction: match dir {
            SortDir::Asc => SortDirection::Asc,
            SortDir::Desc => SortDirection::Desc,
        },
    })
}

/// Map a header click back to a [`SortKey`]. `DataTable` cycles a column
/// Asc→Desc→None; we keep a **2-state Asc/Desc** cycle by treating the `None` step
/// as "return to Asc of the currently-sorted column" (a mode is left untouched, so
/// clicking a header out of a mode starts a fresh column sort).
fn from_sort_state(next: Option<SortState>, current: SortKey) -> SortKey {
    match next {
        Some(st) => {
            let d = match st.direction {
                SortDirection::Asc => SortDir::Asc,
                SortDirection::Desc => SortDir::Desc,
            };
            match st.column_id.as_str() {
                "url" => SortKey::Url(d),
                "updated" => SortKey::Updated(d),
                _ => SortKey::Name(d),
            }
        }
        None => match current {
            SortKey::Name(_) => SortKey::Name(SortDir::Asc),
            SortKey::Url(_) => SortKey::Url(SortDir::Asc),
            SortKey::Updated(_) => SortKey::Updated(SortDir::Asc),
            other => other,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{from_sort_state, to_sort_state};
    use crate::features::vault::vault_filters::{SortDir, SortKey};
    use vedge_ui::components::data_display::SortState;
    use vedge_ui::primitives::tokens::SortDirection;

    #[test]
    fn column_sorts_map_to_a_header_state_modes_do_not() {
        assert_eq!(
            to_sort_state(SortKey::Url(SortDir::Desc)),
            Some(SortState {
                column_id: "url".into(),
                direction: SortDirection::Desc,
            })
        );
        assert!(to_sort_state(SortKey::RecentlyUsed).is_none());
        assert!(to_sort_state(SortKey::Custom).is_none());
    }

    #[test]
    fn header_click_is_a_two_state_asc_desc_cycle() {
        // Asc → Desc (Some), Desc → None → back to Asc (never leaves the column).
        let asc = SortKey::Updated(SortDir::Asc);
        let desc = from_sort_state(to_sort_state(SortKey::Updated(SortDir::Desc)), asc);
        assert_eq!(desc, SortKey::Updated(SortDir::Desc));
        let back = from_sort_state(None, SortKey::Updated(SortDir::Desc));
        assert_eq!(back, SortKey::Updated(SortDir::Asc));
    }

    #[test]
    fn clicking_a_header_out_of_a_mode_starts_a_fresh_column_sort() {
        let st = SortState {
            column_id: "name".into(),
            direction: SortDirection::Asc,
        };
        assert_eq!(
            from_sort_state(Some(st), SortKey::RecentlyUsed),
            SortKey::Name(SortDir::Asc)
        );
        // A `None` while in a mode leaves the mode untouched.
        assert_eq!(from_sort_state(None, SortKey::Custom), SortKey::Custom);
    }
}
