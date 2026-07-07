//! Tag-assignment widget for the **detail panel** (read view, any entry type).
//!
//! Tagging is deliberately decoupled from the edit form: it persists directly
//! through `api::entry::set_tags` (backend decrypt→set→re-encrypt) so it works
//! for non-editable types too (Document), and never reveals the payload to WASM.
//! Changes flow through the page's `on_tags` callback (optimistic `items`
//! update + persist); creating a tag inline also pings `on_catalog` so the new
//! tag resolves + leaves the "available" pool.

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
pub fn TagAssign(
    /// The entry being tagged.
    entry_id: String,
    /// The entry's current tag ids (a snapshot; the widget re-mounts with a
    /// fresh value each time the page's `items` refresh after a change).
    initial: Vec<String>,
    /// Ordered tag catalog for chip resolution + the "add existing" list.
    #[prop(into)]
    catalog: Signal<Vec<TagMetaDto>>,
    /// `(entry_id, new_tag_ids)` — persist the entry's assignments.
    on_tags: Callback<(String, Vec<String>)>,
    /// Reload the tag catalog (after an inline create).
    on_catalog: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let id_sv = StoredValue::new(entry_id);
    let ids_sv = StoredValue::new(initial);
    let new_name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Assigned tags → removable chips (unresolved ids from deleted tags skipped;
    // color honored as text tint when present).
    let chips = move || {
        let cat = catalog.get();
        ids_sv
            .get_value()
            .into_iter()
            .filter_map(|id| {
                let meta = cat.iter().find(|t| t.id == id)?;
                let name = meta.name.clone();
                let style = meta
                    .color
                    .clone()
                    .map(|c| format!("color:{c}"))
                    .unwrap_or_default();
                let remove_id = id.clone();
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
                                on_tags
                                    .run((
                                        id_sv.get_value(),
                                        toggle_tag(&ids_sv.get_value(), &remove_id),
                                    ));
                            }
                        >
                            <span aria-hidden="true">"✕"</span>
                        </button>
                    </span>
                })
            })
            .collect_view()
    };

    // Attach an existing catalog tag (rebuilt so the option list tracks the
    // catalog + current assignment).
    let add_existing = move || {
        let cat = catalog.get();
        let assigned = ids_sv.get_value();
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
                        on_tags.run((id_sv.get_value(), toggle_tag(&ids_sv.get_value(), &id)));
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
            let dto = CreateTagDto { name, color: None };
            match api::tag::create_tag(&vault_path, &dto).await {
                Ok(new_id) => {
                    new_name.set(String::new());
                    let mut ids = ids_sv.get_value();
                    if !ids.iter().any(|x| x == &new_id) {
                        ids.push(new_id);
                    }
                    on_tags.run((id_sv.get_value(), ids));
                    on_catalog.run(());
                }
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-2">
            <span class="text-foreground/50 text-xs uppercase tracking-wider">
                {move || t!(i18n, vault.tags_label)}
            </span>
            <Show when=move || !ids_sv.get_value().is_empty()>
                <div class="flex flex-wrap gap-1.5">{chips}</div>
            </Show>
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
