//! Tag-assignment widget for the entry form.
//!
//! Edits `EntryFormData.tag_ids` (persisted by the existing create/update save
//! path) — it does **not** call any per-entry command itself. Renders the
//! currently-assigned tags as removable chips, lets the user attach an existing
//! catalog tag, and creates a new tag inline (`api::tag::create_tag`) then
//! attaches it. The tag catalog is self-loaded on mount so the widget needs no
//! extra props threaded through `EntryForm`.

use super::entry_form::EntryFormData;
use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{CreateTagDto, TagMetaDto};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{Size, Variant};

/// Toggle `id` in a tag-id list: present → remove, absent → append. Order-stable
/// for the kept ids; idempotent per direction.
#[must_use]
pub fn toggle_tag(ids: &[String], id: &str) -> Vec<String> {
    if ids.iter().any(|x| x == id) {
        ids.iter().filter(|x| x.as_str() != id).cloned().collect()
    } else {
        let mut out = ids.to_vec();
        out.push(id.to_owned());
        out
    }
}

/// Catalog tags not yet assigned to the entry (the pool the "add" control offers).
#[must_use]
pub fn available_tags(catalog: &[TagMetaDto], assigned: &[String]) -> Vec<TagMetaDto> {
    catalog
        .iter()
        .filter(|t| !assigned.iter().any(|a| a == &t.id))
        .cloned()
        .collect()
}

#[component]
pub fn TagAssign(data: RwSignal<EntryFormData>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let catalog = RwSignal::new(Vec::<TagMetaDto>::new());
    let new_name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Self-load the tag catalog once on mount. Untracked path read (no owner in
    // the component body); the `spawn_local` body only `set`s.
    let vault_path = active.path.get_untracked().unwrap_or_default();
    if !vault_path.is_empty() {
        spawn_local(async move {
            if let Ok(list) = api::vault::list_tags(&vault_path).await {
                catalog.set(list);
            }
        });
    }

    // Assigned tags → removable chips (unresolved ids from deleted tags are
    // silently skipped; the color is honored as text tint when present).
    let chips = move || {
        let cat = catalog.get();
        data.with(|d| d.tag_ids.clone())
            .into_iter()
            .filter_map(|id| {
                let meta = cat.iter().find(|t| t.id == id)?;
                let name = meta.name.clone();
                let style = meta
                    .color
                    .clone()
                    .map(|c| format!("color:{c}"))
                    .unwrap_or_default();
                let id_for_remove = id.clone();
                Some(view! {
                    <span
                        class="inline-flex items-center gap-1 rounded-full bg-primary/10 text-primary text-xs px-2 py-0.5"
                        style=style
                    >
                        <span class="truncate max-w-[10rem]">{name}</span>
                        <button
                            type="button"
                            class="leading-none hover:text-danger"
                            aria-label=move || t_string!(i18n, vault.tag_remove).to_string()
                            on:click=move |_: web_sys::MouseEvent| {
                                data.update(|d| d.tag_ids = toggle_tag(&d.tag_ids, &id_for_remove));
                            }
                        >
                            <span aria-hidden="true">"✕"</span>
                        </button>
                    </span>
                })
            })
            .collect_view()
    };

    // Attach an existing catalog tag. Rebuilt in a closure so the option list
    // reflects catalog + assignment changes (Select captures options by value).
    let add_existing = move || {
        let cat = catalog.get();
        let assigned = data.with(|d| d.tag_ids.clone());
        let avail = available_tags(&cat, &assigned);
        if avail.is_empty() {
            ().into_any()
        } else {
            let options: Vec<SelectItem> = avail
                .into_iter()
                .map(|t| SelectItem::option(t.id, t.name))
                .collect();
            view! {
                <Select
                    options=options
                    value=Signal::derive(String::new)
                    placeholder=Signal::derive(move || t_string!(i18n, vault.tag_add).to_string())
                    on_change=Callback::new(move |id: String| {
                        data.update(|d| d.tag_ids = toggle_tag(&d.tag_ids, &id));
                    })
                />
            }
            .into_any()
        }
    };

    // Create a new catalog tag inline, then attach it.
    let create = move |_: web_sys::MouseEvent| {
        if busy.get() {
            return;
        }
        let name = new_name.get().trim().to_owned();
        if name.is_empty() {
            return;
        }
        let vault_path = active.path.get().unwrap_or_default();
        let err_prefix = t_string!(i18n, vault.err_tag_create).to_string();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let dto = CreateTagDto {
                name: name.clone(),
                color: None,
            };
            match api::tag::create_tag(&vault_path, &dto).await {
                Ok(id) => {
                    catalog.update(|c| {
                        c.push(TagMetaDto {
                            id: id.clone(),
                            name,
                            color: None,
                            sort_order: 0,
                        });
                    });
                    data.update(|d| {
                        if !d.tag_ids.iter().any(|x| x == &id) {
                            d.tag_ids.push(id);
                        }
                    });
                    new_name.set(String::new());
                }
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-2">
            <span class="text-xs uppercase tracking-wider text-foreground/50">
                {move || t!(i18n, vault.tags_label)}
            </span>
            <div class="flex flex-wrap gap-1.5">{chips}</div>
            <div class="flex gap-2 items-center">
                <div class="flex-1">{add_existing}</div>
            </div>
            <div class="flex gap-2 items-center">
                <div class="flex-1">
                    <Input
                        id="tag-assign-new"
                        value=Signal::derive(move || new_name.get())
                        placeholder=Signal::derive(move || t_string!(i18n, vault.tag_new).to_string())
                        on_input=Callback::new(move |v: String| new_name.set(v))
                    />
                </div>
                {move || {
                    let disabled = busy.get() || new_name.with(|s| s.trim().is_empty());
                    view! {
                        <Button
                            variant=Variant::Secondary
                            size=Size::Sm
                            disabled=disabled
                            on:click=create
                        >
                            {move || t!(i18n, vault.tag_create)}
                        </Button>
                    }
                }}
            </div>
            {move || {
                error
                    .get()
                    .map(|e| view! {
                        <p class="text-sm" style="color:var(--color-danger-text)">{e}</p>
                    })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{available_tags, toggle_tag};
    use vedge_ipc::TagMetaDto;

    fn tag(id: &str, name: &str) -> TagMetaDto {
        TagMetaDto {
            id: id.to_owned(),
            name: name.to_owned(),
            color: None,
            sort_order: 0,
        }
    }

    #[test]
    fn toggle_tag_adds_when_absent() {
        let ids = vec!["a".to_owned()];
        assert_eq!(toggle_tag(&ids, "b"), vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn toggle_tag_removes_when_present() {
        let ids = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(toggle_tag(&ids, "a"), vec!["b".to_owned()]);
    }

    #[test]
    fn toggle_tag_add_then_remove_is_identity() {
        let ids = vec!["a".to_owned()];
        let added = toggle_tag(&ids, "b");
        assert_eq!(toggle_tag(&added, "b"), ids);
    }

    #[test]
    fn available_tags_excludes_assigned() {
        let catalog = vec![tag("a", "Alpha"), tag("b", "Beta"), tag("c", "Gamma")];
        let assigned = vec!["b".to_owned()];
        let got: Vec<String> = available_tags(&catalog, &assigned)
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(got, vec!["a".to_owned(), "c".to_owned()]);
    }
}
