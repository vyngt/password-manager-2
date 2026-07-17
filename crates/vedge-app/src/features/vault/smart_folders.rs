//! Smart / saved folders — named presets of the search / filter / sort view.
//!
//! A preset is the page's live filter + sort state plus an id + name. This is
//! **frontend-only** UI state, persisted per-vault in the generic `app_settings`
//! KV store (key `smartfolders.<vault_path>`), *not* the encrypted vault — a
//! preset references vault-local tag / folder ids, so it's keyed by vault path.
//! Selecting a preset applies its filters to the toolbar signals (`vault.rs`).

use super::folder_tree::FolderScope;
use super::vault_filters::{Filters, SavedSort, SortKey, TagMatch, saved_from};
use crate::i18n::{t, t_string, use_i18n};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use serde::{Deserialize, Serialize};
use vedge_ipc::EntryTypeDto;
use vedge_ui::components::{IconButton, Input};
use vedge_ui::primitives::tokens::{Size, Variant};

/// A saved view: the toolbar filter + sort state, plus an id + display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartFolder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub entry_type: Option<EntryTypeDto>,
    /// Legacy single-tag field (pre-5.3.1d). **Read-only**: migrated into
    /// `tag_ids` on load (see [`apply_preset`]) and never written back, so a
    /// re-saved preset drops it. Kept only so old presets still resolve their tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default)]
    pub tag_match: TagMatch,
    #[serde(default)]
    pub favorites_only: bool,
    #[serde(default)]
    pub scope: FolderScope,
    #[serde(default)]
    pub sort: SavedSort,
}

/// Capture the current toolbar state as a named preset (`id` supplied by the
/// caller so this stays pure / host-testable).
#[must_use]
pub fn capture_preset(id: String, name: String, filters: &Filters, sort: SortKey) -> SmartFolder {
    SmartFolder {
        id,
        name,
        query: filters.query.clone(),
        entry_type: filters.entry_type.clone(),
        tag_id: None, // new presets write only `tag_ids`
        tag_ids: filters.tag_ids.clone(),
        tag_match: filters.tag_match,
        favorites_only: filters.favorites_only,
        scope: filters.scope.clone(),
        sort: saved_from(sort),
    }
}

/// The `(filters, sort)` a preset applies back to the toolbar.
#[must_use]
pub fn apply_preset(p: &SmartFolder) -> (Filters, SortKey) {
    (
        Filters {
            query: p.query.clone(),
            entry_type: p.entry_type.clone(),
            // Migrate a legacy single `tag_id` into a one-element `tag_ids`; a
            // preset saved after 5.3.1d already carries `tag_ids`.
            tag_ids: if p.tag_ids.is_empty() {
                p.tag_id.clone().into_iter().collect()
            } else {
                p.tag_ids.clone()
            },
            tag_match: p.tag_match,
            favorites_only: p.favorites_only,
            scope: p.scope.clone(),
        },
        SortKey::from(p.sort),
    )
}

/// The "Smart" sidebar section: a compact `+` to save the current view, then the
/// saved presets — click to apply, hover to rename / delete. Mirrors the folder
/// tree's row affordances so the two sections feel like one sidebar.
#[component]
pub fn SmartFolders(
    #[prop(into)] presets: Signal<Vec<SmartFolder>>,
    /// Apply a preset's filters to the toolbar.
    on_apply: Callback<SmartFolder>,
    /// Save the current view under `name` (the page mints the id).
    on_save: Callback<String>,
    /// Delete a preset by id.
    on_delete: Callback<String>,
    /// `(id, new_name)` — rename a preset.
    on_rename: Callback<(String, String)>,
) -> impl IntoView {
    let i18n = use_i18n();
    let show_save = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let renaming = RwSignal::new(Option::<String>::None);
    let rename_value = RwSignal::new(String::new());

    let save = move || {
        let name = new_name.get().trim().to_owned();
        if name.is_empty() {
            return;
        }
        on_save.run(name);
        new_name.set(String::new());
        show_save.set(false);
    };

    view! {
        <div class="flex flex-col gap-0.5 mt-2 pt-2 border-t border-border">
            <div class="flex items-center justify-between px-1 py-0.5">
                <span class="text-foreground/50 text-xs uppercase tracking-wider">
                    {move || t!(i18n, vault.smart_folders_label)}
                </span>
                <IconButton
                    variant=Variant::Ghost
                    size=Size::Xs
                    class="text-foreground/40"
                    aria_label=Signal::derive(move || t_string!(i18n, vault.smart_save).to_owned())
                    on:click=move |_: web_sys::MouseEvent| show_save.update(|v| *v = !*v)
                >
                    <Icon attr:aria-hidden="true" icon=i::FaPlusSolid width="12" height="12" />
                </IconButton>
            </div>

            // Compact "save current view" input, revealed by the header "+".
            <Show when=move || show_save.get()>
                <Input
                    id="smart-save"
                    class="mb-1"
                    autofocus=true
                    placeholder=Signal::derive(move || {
                        t_string!(i18n, vault.smart_name_placeholder).to_owned()
                    })
                    value=Signal::derive(move || new_name.get())
                    on_input=Callback::new(move |v: String| new_name.set(v))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        match ev.key().as_str() {
                            "Enter" => {
                                ev.prevent_default();
                                save();
                            }
                            "Escape" => {
                                ev.prevent_default();
                                new_name.set(String::new());
                                show_save.set(false);
                            }
                            _ => {}
                        }
                    }
                />
            </Show>

            <Show when=move || presets.get().is_empty() && !show_save.get()>
                <p class="px-2 py-1 text-xs text-foreground/40">
                    {move || t!(i18n, vault.smart_empty)}
                </p>
            </Show>

            <For
                each=move || presets.get()
                key=|p| (p.id.clone(), p.name.clone())
                children=move |p| {
                    let apply_p = p.clone();
                    let rn_check_id = p.id.clone();
                    let start_id = p.id.clone();
                    let start_nm = p.name.clone();
                    let del_id = p.id.clone();
                    let name = p.name;
                    let commit = move || {
                        if let Some(id) = renaming.get_untracked() {
                            on_rename.run((id, rename_value.get_untracked()));
                        }
                        renaming.set(None);
                    };
                    // Commit an inline rename (shared by Enter + blur).
                    view! {
                        <div class="group flex items-center gap-1 w-full px-1 py-1.5 rounded text-sm cursor-pointer hover:bg-primary/5">
                            <span class="flex shrink-0 text-foreground/50">
                                <Icon
                                    attr:aria-hidden="true"
                                    icon=i::FaFilterSolid
                                    width="12"
                                    height="12"
                                />
                            </span>
                            {move || {
                                if renaming.get().as_deref() == Some(rn_check_id.as_str()) {
                                    leptos::either::Either::Left(
                                        view! {
                                            <Input
                                                id="smart-rename"
                                                class="flex-1 min-w-0"
                                                autofocus=true
                                                value=Signal::derive(move || rename_value.get())
                                                on_input=Callback::new(move |v: String| rename_value.set(v))
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                }
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    match ev.key().as_str() {
                                                        "Enter" => {
                                                            ev.prevent_default();
                                                            commit();
                                                        }
                                                        "Escape" => {
                                                            ev.prevent_default();
                                                            renaming.set(None);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                on:focusout=move |_: web_sys::FocusEvent| commit()
                                            />
                                        },
                                    )
                                } else {
                                    let name = name.clone();
                                    let name_title = name.clone();
                                    let apply_p = apply_p.clone();
                                    leptos::either::Either::Right(
                                        view! {
                                            <span
                                                class="flex-1 truncate"
                                                title=name_title
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    on_apply.run(apply_p.clone());
                                                }
                                            >
                                                {name}
                                            </span>
                                        },
                                    )
                                }
                            }}
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Xs
                                class="shrink-0 opacity-0 group-hover:opacity-100 text-foreground/40"
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, vault.smart_rename).to_owned()
                                })
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    renaming.set(Some(start_id.clone()));
                                    rename_value.set(start_nm.clone());
                                }
                            >
                                <Icon
                                    attr:aria-hidden="true"
                                    icon=i::FaPenSolid
                                    width="10"
                                    height="10"
                                />
                            </IconButton>
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Xs
                                class="shrink-0 opacity-0 group-hover:opacity-100 text-foreground/40 hover:text-danger"
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, vault.smart_delete).to_owned()
                                })
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_delete.run(del_id.clone());
                                }
                            >
                                <Icon
                                    attr:aria-hidden="true"
                                    icon=i::BiTrashRegular
                                    width="10"
                                    height="10"
                                />
                            </IconButton>
                        </div>
                    }
                }
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::vault::vault_filters::SortDir;

    fn sample_filters() -> Filters {
        Filters {
            query: "aws".into(),
            entry_type: Some(EntryTypeDto::Login),
            tag_ids: vec!["t1".into(), "t2".into()],
            tag_match: TagMatch::All,
            favorites_only: true,
            scope: FolderScope::Folder("f1".into()),
        }
    }

    #[test]
    fn capture_then_apply_round_trips_the_view() {
        let f = sample_filters();
        let preset = capture_preset(
            "id1".into(),
            "AWS logins".into(),
            &f,
            SortKey::Updated(SortDir::Desc),
        );
        let (back, sort) = apply_preset(&preset);
        assert_eq!(back.query, f.query);
        assert_eq!(back.entry_type, f.entry_type);
        assert_eq!(back.tag_ids, f.tag_ids);
        assert_eq!(back.tag_match, f.tag_match);
        assert_eq!(back.favorites_only, f.favorites_only);
        assert_eq!(back.scope, f.scope);
        assert_eq!(sort, SortKey::Updated(SortDir::Desc));
        // A new preset writes only `tag_ids`, never the legacy `tag_id`.
        assert!(preset.tag_id.is_none());
    }

    #[test]
    fn smart_folder_serde_round_trips() {
        // Custom is coerced to Name(Asc) on capture (a preset can't be Custom, ⑪).
        let preset = capture_preset(
            "id1".into(),
            "AWS".into(),
            &sample_filters(),
            SortKey::Custom,
        );
        assert_eq!(preset.sort, SavedSort::Name(SortDir::Asc));
        let json = serde_json::to_string(&preset).unwrap();
        let back: SmartFolder = serde_json::from_str(&json).unwrap();
        assert_eq!(back, preset);
    }

    #[test]
    fn missing_optional_fields_default_on_load() {
        // A minimal stored preset (only id + name) loads with empty/default facets.
        let json = r#"{"id":"x","name":"Everything"}"#;
        let back: SmartFolder = serde_json::from_str(json).unwrap();
        assert_eq!(back.query, "");
        assert!(back.entry_type.is_none());
        assert!(back.tag_ids.is_empty());
        assert_eq!(back.tag_match, TagMatch::Any);
        assert!(!back.favorites_only);
        assert_eq!(back.scope, FolderScope::All);
        assert_eq!(back.sort, SavedSort::Name(SortDir::Asc));
    }

    // 🔴 ⑧/⑪ (spec test #6d): a preset saved with the OLD shape — a single
    // `tag_id` and the fieldless `"Manual"` sort — loads and *applies* as a
    // one-element `tag_ids` and `Name(Asc)`. Nothing is silently dropped.
    #[test]
    fn old_shape_preset_migrates_on_load() {
        let json = r#"{"id":"x","name":"Old","tag_id":"t1","sort":"Manual"}"#;
        let back: SmartFolder = serde_json::from_str(json).unwrap();
        // The legacy field is read; `tag_ids` is still empty until apply merges it.
        assert_eq!(back.tag_id.as_deref(), Some("t1"));
        assert_eq!(back.sort, SavedSort::Name(SortDir::Asc)); // "Manual" coerced

        let (filters, sort) = apply_preset(&back);
        assert_eq!(filters.tag_ids, vec!["t1".to_owned()]);
        assert_eq!(sort, SortKey::Name(SortDir::Asc));
    }
}
