use super::entry_view::{short_date, type_label_i18n};
use crate::i18n::{t, t_string, use_i18n};
use leptos::either::Either;
use leptos::prelude::*;
use std::collections::HashMap;
use vedge_ipc::{IndexEntryDto, TagMetaDto};
use vedge_ui::components::empty_state::EmptyState;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use icondata as i;
use leptos_icons::Icon;

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
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex-1 min-w-0 overflow-auto">
            <Show
                when=move || !items.get().is_empty()
                fallback=move || {
                    view! {
                        <div class="flex items-center justify-center h-full">
                            <EmptyState icon=i::FaInboxSolid title=empty_label />
                        </div>
                    }
                }
            >
                <table class="w-full table-auto">
                    <thead>
                        <tr class="border-b border-secondary/20 text-left text-foreground/60 text-xs uppercase tracking-wider">
                            <th class="p-3 w-[40px]"></th>
                            <th class="p-3">{move || t!(i18n, vault.col_name)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_type)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_url)}</th>
                            <th class="p-3">{move || t!(i18n, vault.col_updated)}</th>
                            <th class="p-3 w-[80px]">{move || t!(i18n, vault.col_actions)}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || items.get()
                            // Key includes the mutable display fields so an
                            // optimistic favorite/tag flip or a post-edit
                            // `updated_at` change re-renders the row — a keyed
                            // `<For>` otherwise keeps stale children for a fixed key.
                            key=|item| {
                                (
                                    item.id.clone(),
                                    item.is_favorite,
                                    item.tag_ids.join(","),
                                    item.folder_id.clone(),
                                    item.sort_order,
                                    item.updated_at.clone(),
                                )
                            }
                            children=move |item| {
                                view! {
                                    <VaultTableRow
                                        item=item
                                        hide_delete=hide_delete
                                        tags=tags
                                        on_delete=on_delete
                                        on_select=on_select
                                        on_favorite=on_favorite
                                        on_move_request=on_move_request
                                        on_reorder=on_reorder
                                        reorder_enabled=reorder_enabled
                                    />
                                }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

#[component]
fn VaultTableRow(
    item: IndexEntryDto,
    hide_delete: Signal<bool>,
    tags: Signal<HashMap<String, TagMetaDto>>,
    on_delete: Callback<String>,
    on_select: Callback<IndexEntryDto>,
    on_favorite: Callback<(String, bool)>,
    on_move_request: Callback<IndexEntryDto>,
    on_reorder: Callback<(String, String)>,
    reorder_enabled: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();

    let entry_for_select = item.clone();
    let entry_for_move = item.clone();
    let item_id = item.id.clone();
    // Stable per-entry hook for the e2e harness (slice 2.9.2) — lets a test
    // address a specific row without matching on (localized) cell text.
    let row_id = item.id.clone();
    let drag_id = item.id.clone();
    // This row as a reorder drop target: the dragged entry's id + the highlight.
    let drop_target_id = item.id.clone();
    let drag_over = RwSignal::new(false);
    let fav_id = item.id.clone();
    let is_fav = item.is_favorite;
    let tag_ids = item.tag_ids.clone();
    let name = item.name.clone();
    let name_title = name.clone();
    // Reactive so it doesn't read the i18n locale in the (owner-less) row body
    // and relocalizes on language switch.
    let entry_type = item.entry_type.clone();
    let type_lbl = Signal::derive(move || type_label_i18n(i18n, &entry_type));
    let url = item.url.clone().unwrap_or_default();
    let url_title = url.clone();
    let updated = short_date(&item.updated_at);

    // Resolve this row's tag ids to name + color chips (unresolved ids skipped).
    let chips = move || {
        let map = tags.get();
        tag_ids
            .iter()
            .filter_map(|id| {
                let meta = map.get(id)?;
                let style = meta
                    .color
                    .clone()
                    .map(|c| format!("color:{c}"))
                    .unwrap_or_default();
                Some(view! {
                    <span
                        class="inline-flex items-center rounded-full bg-primary/10 text-primary text-[10px] px-1.5 py-0.5"
                        style=style
                    >
                        {meta.name.clone()}
                    </span>
                })
            })
            .collect_view()
    };

    view! {
        <tr
            data-entry-row="true"
            data-entry-id=row_id
            class="border-b border-secondary/10 hover:bg-primary/5 transition-colors cursor-pointer"
            class=("border-t-2", move || drag_over.get())
            class=("border-t-primary", move || drag_over.get())
            draggable="true"
            on:click=move |_: web_sys::MouseEvent| on_select.run(entry_for_select.clone())
            on:dragstart=move |ev: web_sys::DragEvent| {
                if let Some(dt) = ev.data_transfer() {
                    let _ = dt.set_data("text/plain", &drag_id);
                }
            }
            // Reorder drop target — active only under the Manual sort, so a plain
            // move-to-folder drag isn't hijacked. Not preventing default under the
            // other sorts means the row simply isn't a drop target there.
            on:dragover=move |ev: web_sys::DragEvent| {
                if reorder_enabled.get() {
                    ev.prevent_default();
                }
            }
            on:dragenter=move |_: web_sys::DragEvent| {
                if reorder_enabled.get_untracked() {
                    drag_over.set(true);
                }
            }
            on:dragleave=move |_: web_sys::DragEvent| drag_over.set(false)
            on:drop=move |ev: web_sys::DragEvent| {
                drag_over.set(false);
                if !reorder_enabled.get_untracked() {
                    return;
                }
                ev.prevent_default();
                let moved = ev
                    .data_transfer()
                    .and_then(|dt| dt.get_data("text/plain").ok())
                    .filter(|s| !s.is_empty());
                if let Some(moved) = moved {
                    if moved != drop_target_id {
                        on_reorder.run((moved, drop_target_id.clone()));
                    }
                }
            }
        >
            <td class="p-3">
                <IconButton
                    aria_label=Signal::derive(move || {
                        if is_fav {
                            t_string!(i18n, vault.unfavorite).to_owned()
                        } else {
                            t_string!(i18n, vault.favorite).to_owned()
                        }
                    })
                    variant=Variant::Ghost
                    size=Size::Sm
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        on_favorite.run((fav_id.clone(), !is_fav));
                    }
                >
                    {if is_fav {
                        Either::Left(view! { <Icon attr:aria-hidden="true" icon=i::FaStarSolid /> })
                    } else {
                        Either::Right(
                            view! { <Icon attr:aria-hidden="true" icon=i::FaStarRegular /> },
                        )
                    }}
                </IconButton>
            </td>
            <td class="p-3 text-sm">
                <div class="flex flex-col gap-1 min-w-0">
                    <span class="block max-w-[16rem] truncate" title=name_title>
                        {name}
                    </span>
                    <div class="flex flex-wrap gap-1">{chips}</div>
                </div>
            </td>
            <td class="p-3 text-sm text-foreground/70">{type_lbl}</td>
            <td
                class="p-3 text-sm font-jetbrains-mono text-foreground/60 max-w-[14rem] truncate"
                title=url_title
            >
                {url}
            </td>
            <td class="p-3 text-sm text-foreground/60">{updated}</td>
            <td class="p-3">
                <Show when=move || {
                    !hide_delete.get()
                }>
                    {
                        let item_id = item_id.clone();
                        let entry_for_move = entry_for_move.clone();
                        // Clone per render so the inner `on:click` (a `move`
                        // closure) doesn't take ownership out of the `Show`'s
                        // re-runnable `Fn` children.
                        view! {
                            <div class="flex items-center gap-1">
                                <IconButton
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.folder_move).to_owned()
                                    })
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_move_request.run(entry_for_move.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::FaFolderOpenSolid />
                                </IconButton>
                                <IconButton
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.delete).to_owned()
                                    })
                                    variant=Variant::Danger
                                    size=Size::Sm
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        on_delete.run(item_id.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::BiTrashRegular />
                                </IconButton>
                            </div>
                        }
                    }
                </Show>
            </td>
        </tr>
    }
}
