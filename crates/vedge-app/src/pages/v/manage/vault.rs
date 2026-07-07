use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::document_attach::DocumentAttach;
use crate::features::vault::entry_form::EntryFormData;
use crate::features::vault::folder_delete::FolderDelete;
use crate::features::vault::folder_move::FolderMove;
use crate::features::vault::folder_tree::{
    FolderBreadcrumb, FolderScope, FolderTree, build_folder_tree, subtree_contents,
};
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
use vedge_ui::primitives::tokens::{Size, Variant};

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

    // Folder navigation scope (client-side, over the loaded index) + the targets
    // for the move-to-folder picker and the non-empty-folder delete confirm.
    let current_scope = RwSignal::new(FolderScope::All);
    let move_target = RwSignal::new(Option::<IndexEntryDto>::None);
    let folder_delete_target = RwSignal::new(Option::<IndexEntryDto>::None);

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
    // The folder hierarchy, rebuilt from the loaded index (folders are entries).
    let folders = Memo::new(move |_| build_folder_tree(&items.get()));
    // The filtered + sorted list feeding the table.
    let visible = Signal::derive(move || {
        let filters = Filters {
            query: search_query.get(),
            entry_type: entry_type.get(),
            tag_id: tag_id.get(),
            favorites_only: favorites_only.get(),
            scope: current_scope.get(),
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
        // A non-empty folder can't be deleted outright — open the two-stage
        // confirm dialog (empty-first). An empty folder / non-folder deletes
        // directly. Reads are untracked (owner-safe either way).
        let entry = items.with_untracked(|l| l.iter().find(|e| e.id == id).cloned());
        if let Some(e) = &entry {
            if e.entry_type == EntryTypeDto::Folder
                && !subtree_contents(&items.get_untracked(), &folders.get_untracked(), &id)
                    .is_empty()
            {
                folder_delete_target.set(Some(e.clone()));
                return;
            }
        }
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

    // Reload just the tag catalog (no entries refetch, no loading flash) — used
    // by the tag manager and by inline tag creation.
    let refresh_tags = move || {
        let vault_path = untrack(|| active.path.get()).unwrap_or_default();
        if vault_path.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(list) = api::vault::list_tags(&vault_path).await {
                tags.set(list);
            }
        });
    };
    let on_catalog = Callback::new(move |()| refresh_tags());

    // Persist an entry's tag assignments: optimistic in-place `items` flip (no
    // loading flash) then `set_tags`, reverting on error. `tag_ids` is part of
    // the table's `<For>` key so the row repaints. Reads are untracked so it's
    // owner-safe when called from a `spawn_local` (inline tag create) too.
    let on_tags = Callback::new(move |(id, new_ids): (String, Vec<String>)| {
        let prev = items
            .with_untracked(|list| list.iter().find(|e| e.id == id).map(|e| e.tag_ids.clone()));
        items.update(|list| {
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.tag_ids = new_ids.clone();
            }
        });
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_tag_assign).to_string());
        let revert_id = id.clone();
        spawn_local(async move {
            if let Err(e) = api::entry::set_tags(&vault_path, &id, &new_ids).await {
                if let Some(p) = prev {
                    items.update(|list| {
                        if let Some(en) = list.iter_mut().find(|en| en.id == revert_id) {
                            en.tag_ids = p;
                        }
                    });
                }
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

    // Leaving the folder view when toggling active/trash (folders are an active-
    // view concept). Guarded so the initial mount doesn't reset anything.
    Effect::new(move |prev: Option<bool>| {
        let t = trashed_view.get();
        if prev.is_some_and(|p| p != t) {
            current_scope.set(FolderScope::All);
        }
        t
    });

    // Persist a move: optimistic in-place `folder_id` flip (no loading flash) then
    // `move_entry`, reverting on error. `folder_id` is in the table `<For>` key so
    // the row repaints. Reads are untracked (owner-safe from any caller).
    let on_move = Callback::new(move |(id, dest): (String, Option<String>)| {
        let prev =
            items.with_untracked(|l| l.iter().find(|e| e.id == id).map(|e| e.folder_id.clone()));
        items.update(|l| {
            if let Some(e) = l.iter_mut().find(|e| e.id == id) {
                e.folder_id = dest.clone();
            }
        });
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_move).to_string());
        let revert_id = id.clone();
        let dest_owned = dest.clone();
        spawn_local(async move {
            if let Err(e) = api::entry::move_entry(&vault_path, &id, dest_owned.as_deref()).await {
                if let Some(p) = prev {
                    items.update(|l| {
                        if let Some(en) = l.iter_mut().find(|en| en.id == revert_id) {
                            en.folder_id = p;
                        }
                    });
                }
                status_msg.set(Some(format!("{err_prefix}{e}")));
            }
        });
    });

    let on_move_request = Callback::new(move |entry: IndexEntryDto| move_target.set(Some(entry)));

    // The palette's "Move to folder…" bumps `ui.move_request`; open the picker for
    // the selected entry when it changes (prev-guard skips the initial mount).
    Effect::new(move |prev: Option<u32>| {
        let cur = ui.move_request.get();
        if let Some(p) = prev {
            if cur != p {
                if let Some(id) = ui.selected_id.get_untracked() {
                    if let Some(e) =
                        items.with_untracked(|l| l.iter().find(|e| e.id == id).cloned())
                    {
                        move_target.set(Some(e));
                    }
                }
            }
        }
        cur
    });

    // Create a folder in the current folder (or root). Builds the payload through
    // the shared form model so the Folder arm stays the single source of truth.
    let on_new_folder = Callback::new(move |(parent, name): (Option<String>, String)| {
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_folder_create).to_string();
        let mut d = EntryFormData::new(EntryTypeDto::Folder);
        d.name = name;
        d.folder_id = parent;
        let Ok(payload) = d.to_payload() else {
            return;
        };
        spawn_local(async move {
            match api::entry::create_entry(&vault_path, &payload).await {
                Ok(_id) => refresh(),
                Err(e) => status_msg.set(Some(format!("{err_prefix}{e}"))),
            }
        });
    });

    // Recursively soft-delete a folder's contents (the delete dialog's "Empty
    // folder"). Sequential so a mid-way failure surfaces and stops.
    let on_empty = Callback::new(move |ids: Vec<String>| {
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_folder_delete).to_string();
        spawn_local(async move {
            for id in ids {
                if let Err(e) = api::entry::soft_delete_entry(&vault_path, &id).await {
                    status_msg.set(Some(format!("{err_prefix}{e}")));
                    break;
                }
            }
            refresh();
        });
    });

    // Delete the now-empty folder itself (the delete dialog's second stage).
    let on_delete_folder = Callback::new(move |id: String| {
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_folder_delete).to_string();
        let was_selected = ui.selected_id.get().as_deref() == Some(id.as_str());
        let was_scoped = matches!(current_scope.get(), FolderScope::Folder(ref s) if *s == id);
        spawn_local(async move {
            match api::entry::soft_delete_entry(&vault_path, &id).await {
                Ok(()) => {
                    if was_selected {
                        ui.selected_id.set(None);
                    }
                    if was_scoped {
                        current_scope.set(FolderScope::All);
                    }
                    folder_delete_target.set(None);
                    refresh();
                }
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
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    class="whitespace-nowrap"
                    on:click=move |_| manage_tags_open.set(true)
                >
                    {move || t!(i18n, vault.tag_manage)}
                </Button>
            </div>

            <TagManager
                open=manage_tags_open
                tags=Signal::derive(move || tags.get())
                entries=Signal::derive(move || items.get())
                on_changed=on_catalog
            />

            <FolderMove target=move_target folders=folders on_move=on_move />
            <FolderDelete
                target=folder_delete_target
                items=Signal::derive(move || items.get())
                folders=folders
                on_empty=on_empty
                on_delete_folder=on_delete_folder
            />

            {move || status_msg.get().map(|m| view! {
                <p class="text-sm text-text-secondary">{m}</p>
            })}

            <Show when=move || ui.show_create.get()>
                <VaultCreateForm show=ui.show_create on_created=on_created folders=folders />
            </Show>

            <Show when=move || !trashed_view.get()>
                <FolderBreadcrumb folders=folders scope=current_scope />
            </Show>

            <Show when=move || show_attach.get()>
                <DocumentAttach show=show_attach on_attached=on_created />
            </Show>

            <div class="flex-1 flex gap-4 min-h-0">
                <Show when=move || !trashed_view.get()>
                    <FolderTree
                        items=Signal::derive(move || items.get())
                        folders=folders
                        scope=current_scope
                        on_new_folder=on_new_folder
                    />
                </Show>
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
                        on_move_request=on_move_request
                    />
                </Show>
                {move || selected_entry.get().map(|entry| view! {
                    <VaultDetail
                        entry=entry
                        tags=Signal::derive(move || tags.get())
                        on_copy=on_copy
                        on_close=on_close
                        on_saved=on_saved
                        on_favorite=on_favorite
                        on_tags=on_tags
                        on_catalog=on_catalog
                        folders=folders
                        on_move_request=on_move_request
                    />
                })}
            </div>
        </div>
    }
}
