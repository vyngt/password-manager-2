use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::document_attach::DocumentAttach;
use crate::features::vault::tag_manager::TagManager;
use crate::features::vault::ui_state::VaultUiState;
use crate::features::vault::vault_create_form::VaultCreateForm;
use crate::features::vault::vault_detail::VaultDetail;
use crate::features::vault::vault_filters::{Filters, SortKey, VaultFilters, filter_and_sort};
use crate::features::vault::vault_table::VaultTable;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::Button;
use vedge_ui::components::tooltip_icon_button::TooltipIconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn VaultPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let ui = expect_context::<VaultUiState>();

    let items = RwSignal::new(Vec::<IndexEntryDto>::new());
    let loading = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());
    let show_attach = RwSignal::new(false);
    // Selection + create-toggle live in the shared `VaultUiState` (so the command
    // palette can drive them); the selected entry is *derived* from the live
    // `items`, so an edit (which refreshes `items`) auto-updates the open panel
    // instead of showing a stale metadata snapshot.
    let selected_entry = Signal::derive(move || {
        ui.selected_id
            .get()
            .and_then(|id| items.get().into_iter().find(|e| e.id == id))
    });
    let status_msg = RwSignal::new(Option::<String>::None);

    // Filter / sort facets (client-side over the loaded metadata).
    let entry_type = RwSignal::new(Option::<EntryTypeDto>::None);
    let tag_id = RwSignal::new(Option::<String>::None);
    let favorites_only = RwSignal::new(false);
    let sort = RwSignal::new(SortKey::default());
    // Active vs trashed switches the *fetch source* (see `refresh`), not a facet.
    let trashed_view = RwSignal::new(false);
    // Tags back both the tag facet and tag-name search.
    let tags = RwSignal::new(Vec::<TagMetaDto>::new());

    // `tag_id -> name`, memoized so keystroke-level re-filtering doesn't rebuild it.
    let tag_map = Memo::new(move |_| {
        tags.get()
            .into_iter()
            .map(|t| (t.id, t.name))
            .collect::<HashMap<String, String>>()
    });
    // `tag_id -> full meta` (keeps color) for resolving row/detail tag chips.
    let tag_lookup = Memo::new(move |_| {
        tags.get()
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect::<HashMap<String, TagMetaDto>>()
    });
    // Opens the tag-catalog manager modal.
    let manage_tags_open = RwSignal::new(false);
    // The filtered + sorted list feeding the table.
    let visible = Signal::derive(move || {
        let filters = Filters {
            query: search_query.get(),
            entry_type: entry_type.get(),
            tag_id: tag_id.get(),
            favorites_only: favorites_only.get(),
        };
        tag_map.with(|m| filter_and_sort(&items.get(), &filters, m, sort.get()))
    });
    // Distinguish *no entries yet* / *no matches* / *empty trash* when the table
    // is empty (shown by `VaultTable`'s fallback).
    let empty_label = Signal::derive(move || {
        if items.get().is_empty() {
            if trashed_view.get() {
                t_string!(i18n, vault.empty_trash).to_string()
            } else {
                t_string!(i18n, vault.empty_none).to_string()
            }
        } else {
            t_string!(i18n, vault.empty_no_matches).to_string()
        }
    });

    // Fetch the live entry index for the active vault. Re-run on demand
    // (mount, create-ping, post-delete, view switch).
    let refresh = move || {
        // `refresh` runs from an `Effect` *and* from inside `spawn_local`
        // (on create/save/delete pings) — the latter has no reactive owner, so
        // read both the path and the locale string `untrack`ed. The Effect below
        // is what actually tracks `active.path` for re-runs.
        let vault_path = untrack(|| active.path.get()).unwrap_or_default();
        if vault_path.is_empty() {
            items.set(Vec::new());
            tags.set(Vec::new());
            return;
        }
        loading.set(true);
        let is_trashed = untrack(|| trashed_view.get());
        let err_prefix = untrack(|| t_string!(i18n, vault.err_load).to_string());
        let tags_path = vault_path.clone();
        spawn_local(async move {
            // Tags back the facet + tag-name search; best-effort (a failure just
            // leaves the tag facet empty, it doesn't block the entry list).
            if let Ok(list) = api::vault::list_tags(&tags_path).await {
                tags.set(list);
            }
            let result = if is_trashed {
                api::vault::list_trashed(&vault_path).await
            } else {
                api::vault::list_entries(&vault_path).await
            };
            match result {
                Ok(list) => items.set(list),
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
            loading.set(false);
        });
    };

    // Load on mount and whenever the active vault path or the active/trashed
    // view changes (toggling the view re-fetches from the other source).
    Effect::new(move |_| {
        let _ = active.path.get();
        let _ = trashed_view.get();
        refresh();
    });

    let on_delete = Callback::new(move |id: String| {
        // Read signals in the handler body (owner present); inside `spawn_local`
        // they'd be owner-less. `set` is fine there — only reads warn.
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_delete).to_string();
        let was_selected = ui.selected_id.get().as_deref() == Some(id.as_str());
        spawn_local(async move {
            match api::entry::soft_delete_entry(&vault_path, &id).await {
                Ok(()) => {
                    if was_selected {
                        ui.selected_id.set(None);
                    }
                    refresh();
                }
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    });

    let on_select = Callback::new(move |entry: IndexEntryDto| ui.selected_id.set(Some(entry.id)));
    let on_close = Callback::new(move |()| ui.selected_id.set(None));
    let on_created = Callback::new(move |()| refresh());
    let on_saved = Callback::new(move |()| refresh());

    // Favorite toggle: flip `items` in place (optimistic — avoids the whole-table
    // loading flash a full `refresh()` would trigger) then persist via
    // `set_favorite`. `update` is a write, so it's owner-safe in `spawn_local`;
    // on error we revert and surface a message. `is_favorite` is part of the
    // table's `<For>` key, so the row repaints on the in-place flip.
    let on_favorite = Callback::new(move |(id, next): (String, bool)| {
        items.update(|list| {
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.is_favorite = next;
            }
        });
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_favorite).to_string();
        let revert_id = id.clone();
        spawn_local(async move {
            if let Err(e) = api::entry::set_favorite(&vault_path, &id, next).await {
                items.update(|list| {
                    if let Some(en) = list.iter_mut().find(|en| en.id == revert_id) {
                        en.is_favorite = !next;
                    }
                });
                status_msg.set(Some(format!("{err_prefix}{e}")));
            }
        });
    });

    let on_copy = Callback::new(move |field: FieldSelectorDto| {
        let vault_path = active.path.get().unwrap_or_default();
        let Some(id) = ui.selected_id.get() else {
            return;
        };
        // Read the locale string here (reactive owner present); reading it
        // inside `spawn_local` trips the "outside a reactive tracking context"
        // warning.
        let copied_msg = t_string!(i18n, vault.copied).to_string();
        let err_prefix = t_string!(i18n, vault.err_copy).to_string();
        spawn_local(async move {
            match api::entry::copy_field(&vault_path, &id, &field, Some(30)).await {
                Ok(()) => status_msg.set(Some(copied_msg)),
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    });

    view! {
        <div class="h-full flex flex-col gap-4 p-4">
            <div class="flex items-center gap-3">
                <VaultFilters
                    search_query=search_query
                    entry_type=entry_type
                    tag_id=tag_id
                    favorites_only=favorites_only
                    sort=sort
                    trashed_view=trashed_view
                    tags=Signal::derive(move || tags.get())
                />
                <Button
                    variant=Variant::Primary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    on:click=move |_| {
                        show_attach.set(false);
                        ui.show_create.update(|v| *v = !*v);
                    }
                >
                    {move || t!(i18n, vault.new_item)}
                </Button>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    on:click=move |_| {
                        ui.show_create.set(false);
                        show_attach.update(|v| *v = !*v);
                    }
                >
                    {move || t!(i18n, vault.attach_document)}
                </Button>
                <TooltipIconButton
                    label=Signal::derive(move || t_string!(i18n, vault.tag_manage).to_string())
                    on_click=Callback::new(move |()| manage_tags_open.set(true))
                >
                    <Icon icon=i::FaTagsSolid />
                </TooltipIconButton>
            </div>

            <TagManager
                open=manage_tags_open
                tags=Signal::derive(move || tags.get())
                on_changed=on_created
            />

            {move || status_msg.get().map(|m| view! {
                <p class="text-sm text-text-secondary">{m}</p>
            })}

            <Show when=move || ui.show_create.get()>
                <VaultCreateForm show=ui.show_create on_created=on_created />
            </Show>

            <Show when=move || show_attach.get()>
                <DocumentAttach show=show_attach on_attached=on_created />
            </Show>

            <div class="flex-1 flex gap-4 min-h-0">
                <Show
                    when=move || !loading.get()
                    fallback=move || {
                        view! {
                            <div class="flex-1 flex items-center justify-center text-foreground/40 text-sm">
                                {move || t!(i18n, vault.loading)}
                            </div>
                        }
                    }
                >
                    <VaultTable
                        items=visible
                        empty_label=empty_label
                        hide_delete=Signal::derive(move || trashed_view.get())
                        tags=Signal::derive(move || tag_lookup.get())
                        on_delete=on_delete
                        on_select=on_select
                        on_favorite=on_favorite
                    />
                </Show>
                {move || selected_entry.get().map(|entry| view! {
                    <VaultDetail
                        entry=entry
                        tags=Signal::derive(move || tag_lookup.get())
                        on_copy=on_copy
                        on_close=on_close
                        on_saved=on_saved
                        on_favorite=on_favorite
                    />
                })}
            </div>
        </div>
    }
}
