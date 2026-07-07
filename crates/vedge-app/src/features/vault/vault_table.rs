use super::entry_view::{short_date, type_label_i18n};
use crate::i18n::*;
use leptos::either::Either;
use leptos::prelude::*;
use std::collections::HashMap;
use vedge_ipc::{IndexEntryDto, TagMetaDto};
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
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex-1 overflow-auto">
            <Show
                when=move || !items.get().is_empty()
                fallback=move || {
                    view! {
                        <div class="flex items-center justify-center h-full text-foreground/40 text-sm">
                            {move || empty_label.get()}
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
                            // optimistic favorite flip or a post-edit `tag_ids` /
                            // `updated_at` change re-renders the row — a keyed
                            // `<For>` otherwise keeps stale children for a fixed key.
                            key=|item| {
                                (item.id.clone(), item.is_favorite, item.updated_at.clone())
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
) -> impl IntoView {
    let i18n = use_i18n();

    let entry_for_select = item.clone();
    let item_id = item.id.clone();
    let fav_id = item.id.clone();
    let is_fav = item.is_favorite;
    let tag_ids = item.tag_ids.clone();
    let name = item.name.clone();
    // Reactive so it doesn't read the i18n locale in the (owner-less) row body
    // and relocalizes on language switch.
    let entry_type = item.entry_type.clone();
    let type_lbl = Signal::derive(move || type_label_i18n(i18n, &entry_type));
    let url = item.url.clone().unwrap_or_default();
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
            class="border-b border-secondary/10 hover:bg-primary/5 transition-colors cursor-pointer"
            on:click=move |_: web_sys::MouseEvent| on_select.run(entry_for_select.clone())
        >
            <td class="p-3">
                <IconButton
                    aria_label=Signal::derive(move || {
                        if is_fav {
                            t_string!(i18n, vault.unfavorite).to_string()
                        } else {
                            t_string!(i18n, vault.favorite).to_string()
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
                        Either::Left(view! { <Icon icon=i::FaStarSolid /> })
                    } else {
                        Either::Right(view! { <Icon icon=i::FaStarRegular /> })
                    }}
                </IconButton>
            </td>
            <td class="p-3 text-sm">
                <div class="flex flex-col gap-1">
                    <span>{name}</span>
                    <div class="flex flex-wrap gap-1">{chips}</div>
                </div>
            </td>
            <td class="p-3 text-sm text-foreground/70">{type_lbl}</td>
            <td class="p-3 text-sm font-jetbrains-mono text-foreground/60">{url}</td>
            <td class="p-3 text-sm text-foreground/60">{updated}</td>
            <td class="p-3">
                <Show when=move || !hide_delete.get()>
                    {
                        // Clone per render so the inner `on:click` (a `move`
                        // closure) doesn't take ownership out of the `Show`'s
                        // re-runnable `Fn` children.
                        let item_id = item_id.clone();
                        view! {
                            <IconButton
                                aria_label=Signal::derive(move || t_string!(i18n, vault.delete).to_string())
                                variant=Variant::Danger
                                size=Size::Sm
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_delete.run(item_id.clone());
                                }
                            >
                                <Icon icon=i::BiTrashRegular />
                            </IconButton>
                        }
                    }
                </Show>
            </td>
        </tr>
    }
}
