use crate::api;
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::document_attach::DocumentAttach;
use crate::features::vault::entry_form::EntryFormData;
use crate::features::vault::entry_history::EntryHistory;
use crate::features::vault::folder_customize::FolderCustomize;
use crate::features::vault::folder_delete::FolderDelete;
use crate::features::vault::folder_move::FolderMove;
use crate::features::vault::folder_tree::{
    FolderBreadcrumb, FolderScope, FolderTree, build_folder_tree, subtree_contents,
};
use crate::features::vault::smart_folders::{
    SmartFolder, SmartFolders, apply_preset, capture_preset,
};
use crate::features::vault::tag_manager::TagManager;
use crate::features::vault::ui_state::VaultUiState;
use crate::features::vault::vault_create_form::VaultCreateForm;
use crate::features::vault::vault_detail::VaultDetail;
use crate::features::vault::vault_filters::{
    Filters, SortKey, VaultFilters, filter_and_sort, reorder_within,
};
use crate::features::vault::vault_table::VaultTable;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, IndexEntryDto, TagMetaDto};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, Spinner};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};

#[component]
pub fn VaultPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let ui = expect_context::<VaultUiState>();
    let sec = expect_context::<SecurityPrefsCtx>();
    let toast = use_toast();
    // Toast helpers — read the localized dismiss label `untrack`ed so they're
    // safe to call from both event handlers and `spawn_local` futures.
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };

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
    let customize_target = RwSignal::new(Option::<IndexEntryDto>::None);
    let history_target = RwSignal::new(Option::<IndexEntryDto>::None);
    // Saved "smart folder" filter presets (per-vault, persisted in app_settings).
    let smart_folders = RwSignal::new(Vec::<SmartFolder>::new());

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
                Err(e) => show_error(format!("{err_prefix}{e}")),
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

    // Load the saved smart-folder presets for the active vault (client-only state
    // in the `app_settings` KV store, keyed by vault path — not the vault).
    Effect::new(move |_| {
        let vault_path = active.path.get().unwrap_or_default();
        if vault_path.is_empty() {
            smart_folders.set(Vec::new());
            return;
        }
        let key = format!("smartfolders.{vault_path}");
        spawn_local(async move {
            match api::settings::get_app_setting(&key).await {
                Ok(Some(dto)) => {
                    if let Ok(list) = serde_json::from_value::<Vec<SmartFolder>>(dto.value) {
                        smart_folders.set(list);
                    }
                }
                _ => smart_folders.set(Vec::new()),
            }
        });
    });

    // Persist the current preset list back to `app_settings`.
    let persist_smart = Callback::new(move |list: Vec<SmartFolder>| {
        let vault_path = active.path.get_untracked().unwrap_or_default();
        if vault_path.is_empty() {
            return;
        }
        let key = format!("smartfolders.{vault_path}");
        if let Ok(value) = serde_json::to_value(&list) {
            spawn_local(async move {
                let _ = api::settings::set_app_setting(&key, &value).await;
            });
        }
    });

    // Apply a preset: push its captured filter/sort state onto the toolbar signals
    // (and leave the trashed view, since folder scopes are an active-view concept).
    let on_apply_smart = Callback::new(move |p: SmartFolder| {
        let (filters, sk) = apply_preset(&p);
        trashed_view.set(false);
        search_query.set(filters.query);
        entry_type.set(filters.entry_type);
        tag_id.set(filters.tag_id);
        favorites_only.set(filters.favorites_only);
        current_scope.set(filters.scope);
        sort.set(sk);
    });

    // Save the current view as a named preset (the id is minted here so the
    // capture helper stays pure).
    let on_save_smart = Callback::new(move |name: String| {
        let filters = Filters {
            query: search_query.get_untracked(),
            entry_type: entry_type.get_untracked(),
            tag_id: tag_id.get_untracked(),
            favorites_only: favorites_only.get_untracked(),
            scope: current_scope.get_untracked(),
        };
        let id = uuid::Uuid::new_v4().to_string();
        let preset = capture_preset(id, name, &filters, sort.get_untracked());
        smart_folders.update(|l| l.push(preset));
        persist_smart.run(smart_folders.get_untracked());
    });

    let on_delete_smart = Callback::new(move |id: String| {
        smart_folders.update(|l| l.retain(|p| p.id != id));
        persist_smart.run(smart_folders.get_untracked());
    });

    let on_rename_smart = Callback::new(move |(id, name): (String, String)| {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return;
        }
        smart_folders.update(|l| {
            if let Some(p) = l.iter_mut().find(|p| p.id == id) {
                p.name = name.clone();
            }
        });
        persist_smart.run(smart_folders.get_untracked());
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
        // Deleting the folder we're currently scoped into → fall back to All.
        let was_scoped = entry.as_ref().is_some_and(|e| {
            e.entry_type == EntryTypeDto::Folder
                && matches!(current_scope.get(), FolderScope::Folder(ref s) if *s == id)
        });
        spawn_local(async move {
            match api::entry::soft_delete_entry(&vault_path, &id).await {
                Ok(()) => {
                    if was_selected {
                        ui.selected_id.set(None);
                    }
                    if was_scoped {
                        current_scope.set(FolderScope::All);
                    }
                    refresh();
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
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
                show_error(format!("{err_prefix}{e}"));
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
                show_error(format!("{err_prefix}{e}"));
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
        let secs = sec.0.get_untracked().clipboard_clear_seconds;
        spawn_local(async move {
            match api::entry::copy_field(&vault_path, &id, &field, Some(secs)).await {
                Ok(()) => show_success(copied_msg),
                Err(e) => show_error(format!("{err_prefix}{e}")),
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
                show_error(format!("{err_prefix}{e}"));
            }
        });
    });

    // Drag-reorder (Manual sort): renumber the affected run and persist only the
    // changed entries. Optimistic in-place `sort_order` flips (in the `<For>` key,
    // so rows repaint under Manual sort) → `set_sort_order` per changed id, revert
    // on error. Reads are untracked (owner-safe from the row's drop handler).
    let on_reorder = Callback::new(move |(moved, target): (String, String)| {
        // The current display order (id, sort_order) — under Manual this is the
        // folder's contents in shown order.
        let current: Vec<(String, u32)> = visible
            .get_untracked()
            .iter()
            .map(|e| (e.id.clone(), e.sort_order))
            .collect();
        let changes = reorder_within(&current, &moved, &target);
        if changes.is_empty() {
            return;
        }
        // Snapshot the prior orders of just the changed ids for revert.
        let prev: Vec<(String, u32)> = changes
            .iter()
            .filter_map(|(id, _)| {
                items.with_untracked(|l| {
                    l.iter()
                        .find(|e| e.id == *id)
                        .map(|e| (id.clone(), e.sort_order))
                })
            })
            .collect();
        items.update(|l| {
            for (id, ord) in &changes {
                if let Some(e) = l.iter_mut().find(|e| e.id == *id) {
                    e.sort_order = *ord;
                }
            }
        });
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_reorder).to_string());
        spawn_local(async move {
            for (id, ord) in changes {
                if let Err(e) = api::entry::set_sort_order(&vault_path, &id, ord).await {
                    items.update(|l| {
                        for (rid, rord) in &prev {
                            if let Some(en) = l.iter_mut().find(|en| en.id == *rid) {
                                en.sort_order = *rord;
                            }
                        }
                    });
                    show_error(format!("{err_prefix}{e}"));
                    return;
                }
            }
        });
    });

    let on_move_request = Callback::new(move |entry: IndexEntryDto| move_target.set(Some(entry)));
    let on_history_request =
        Callback::new(move |entry: IndexEntryDto| history_target.set(Some(entry)));

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
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // Rename a folder from the tree. A folder carries no secrets, so the
    // get_entry → mutate name → update_entry round-trip reveals nothing sensitive.
    let on_rename_folder = Callback::new(move |(id, name): (String, String)| {
        let name = name.trim().to_owned();
        if name.is_empty() {
            return;
        }
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_folder_rename).to_string();
        spawn_local(async move {
            match api::entry::get_entry(&vault_path, &id).await {
                Ok(payload) => {
                    let mut d = EntryFormData::from_payload(&payload);
                    d.name = name;
                    if let Ok(p) = d.to_payload() {
                        if let Err(e) = api::entry::update_entry(&vault_path, &id, &p).await {
                            show_error(format!("{err_prefix}{e}"));
                        } else {
                            refresh();
                        }
                    }
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    });

    // Open the customize dialog for a folder id (resolve it from the live index).
    let on_customize = Callback::new(move |id: String| {
        if let Some(e) = items.with_untracked(|l| l.iter().find(|e| e.id == id).cloned()) {
            customize_target.set(Some(e));
        }
    });

    // Persist a folder's color/icon. Like rename, a folder carries no secrets, so
    // the get_entry → mutate → update_entry round-trip reveals nothing sensitive.
    let on_customize_apply = Callback::new(
        move |(id, color, icon): (String, Option<String>, Option<String>)| {
            let vault_path = active.path.get().unwrap_or_default();
            let err_prefix = t_string!(i18n, vault.err_folder_customize).to_string();
            spawn_local(async move {
                match api::entry::get_entry(&vault_path, &id).await {
                    Ok(payload) => {
                        let mut d = EntryFormData::from_payload(&payload);
                        d.color = color;
                        d.icon = icon;
                        if let Ok(p) = d.to_payload() {
                            if let Err(e) = api::entry::update_entry(&vault_path, &id, &p).await {
                                show_error(format!("{err_prefix}{e}"));
                            } else {
                                refresh();
                            }
                        }
                    }
                    Err(e) => show_error(format!("{err_prefix}{e}")),
                }
            });
        },
    );

    // Recursively soft-delete a folder's contents (the delete dialog's "Empty
    // folder"). Sequential so a mid-way failure surfaces and stops.
    let on_empty = Callback::new(move |ids: Vec<String>| {
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_folder_delete).to_string();
        spawn_local(async move {
            for id in ids {
                if let Err(e) = api::entry::soft_delete_entry(&vault_path, &id).await {
                    show_error(format!("{err_prefix}{e}"));
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
                Err(e) => show_error(format!("{err_prefix}{e}")),
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
            <FolderCustomize target=customize_target on_apply=on_customize_apply />
            <EntryHistory target=history_target on_restored=on_saved />

            <Show when=move || ui.show_create.get()>
                <VaultCreateForm show=ui.show_create on_created=on_created folders=folders />
            </Show>

            <Show when=move || !trashed_view.get()>
                <FolderBreadcrumb folders=folders scope=current_scope />
            </Show>

            <Show when=move || show_attach.get()>
                <DocumentAttach show=show_attach on_attached=on_created />
            </Show>

            // `relative` so the detail renders as a right-side drawer overlaying
            // the table (see VaultDetail) instead of competing for column width —
            // the folder tree + table now get the full row.
            <div class="relative flex-1 flex gap-4 min-h-0">
                <Show when=move || !trashed_view.get()>
                    <FolderTree
                        items=Signal::derive(move || items.get())
                        folders=folders
                        scope=current_scope
                        on_new_folder=on_new_folder
                        on_move=on_move
                        on_delete=on_delete
                        on_rename=on_rename_folder
                        on_customize=on_customize
                    >
                        <SmartFolders
                            presets=Signal::derive(move || smart_folders.get())
                            on_apply=on_apply_smart
                            on_save=on_save_smart
                            on_delete=on_delete_smart
                            on_rename=on_rename_smart
                        />
                    </FolderTree>
                </Show>
                <Show
                    when=move || !loading.get()
                    fallback=move || {
                        view! {
                            <div class="flex-1 flex items-center justify-center">
                                <Spinner />
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
                        on_reorder=on_reorder
                        reorder_enabled=Signal::derive(move || sort.get() == SortKey::Manual)
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
                        on_history_request=on_history_request
                    />
                })}
            </div>
        </div>
    }
}
