//! Tag catalog manager — create / rename / delete tags.
//!
//! A `Dialog` over the tag catalog (`tags` prop, sourced from the page's single
//! `list_tags` fetch). Every mutation calls the existing `api::tag::*` command
//! then pings `on_changed` (the page's refresh), which re-runs `list_tags` and
//! flows the updated catalog back through the `tags` prop — so the list, the
//! filter facet, and the row/detail chips all stay in sync.
//!
//! Deleting a tag leaves dangling `tag_ids` on entries; the index tolerates
//! that and chips simply skip unresolved ids, so no entry rewrite is needed.

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{CreateTagDto, RenameTagDto, TagMetaDto};
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

#[component]
pub fn TagManager(
    open: RwSignal<bool>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let new_name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let close = Callback::new(move |()| open.set(false));

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
                Ok(_id) => {
                    new_name.set(String::new());
                    on_changed.run(());
                }
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.tag_manager_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-3">
                    <div class="flex gap-2 items-center">
                        <div class="flex-1">
                            <Input
                                id="tag-manager-new"
                                value=Signal::derive(move || new_name.get())
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, vault.tag_new).to_string()
                                })
                                on_input=Callback::new(move |v: String| new_name.set(v))
                            />
                        </div>
                        {move || {
                            let disabled = busy.get() || new_name.with(|s| s.trim().is_empty());
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    size=Size::Sm
                                    disabled=disabled
                                    on:click=create
                                >
                                    {move || t!(i18n, vault.tag_create)}
                                </Button>
                            }
                        }}
                    </div>

                    <Show when=move || tags.get().is_empty()>
                        <p class="text-sm text-foreground/50">
                            {move || t!(i18n, vault.tag_empty)}
                        </p>
                    </Show>

                    <div class="flex flex-col divide-y divide-secondary/15">
                        <For
                            each=move || tags.get()
                            // Key on (id, name) so a rename (same id, new name)
                            // re-creates the row — a keyed `<For>` otherwise keeps
                            // the stale child for a fixed key.
                            key=|t| (t.id.clone(), t.name.clone())
                            children=move |t| view! { <TagManagerRow tag=t on_changed=on_changed /> }
                        />
                    </div>

                    {move || {
                        error
                            .get()
                            .map(|e| view! {
                                <p class="text-sm" style="color:var(--color-danger-text)">{e}</p>
                            })
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}

#[component]
fn TagManagerRow(tag: TagMetaDto, on_changed: Callback<()>) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let tag_id = StoredValue::new(tag.id.clone());
    let tag_name = tag.name.clone();
    // `Copy` handle to the name so `start_rename` stays a `Copy` closure (it's
    // used inside a re-runnable view closure, which can't move a `String` out).
    let name_sv = StoredValue::new(tag.name.clone());

    let editing = RwSignal::new(false);
    let confirming = RwSignal::new(false);
    let edit_name = RwSignal::new(tag.name.clone());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let start_rename = move |_: web_sys::MouseEvent| {
        edit_name.set(name_sv.get_value());
        error.set(None);
        editing.set(true);
    };
    let cancel_rename = move |_: web_sys::MouseEvent| editing.set(false);

    let save_rename = move |_: web_sys::MouseEvent| {
        if busy.get() {
            return;
        }
        let new = edit_name.get().trim().to_owned();
        if new.is_empty() {
            return;
        }
        let vault_path = active.path.get().unwrap_or_default();
        let id = tag_id.get_value();
        let err_prefix = t_string!(i18n, vault.err_tag_rename).to_string();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let dto = RenameTagDto {
                tag_id: id,
                new_name: new,
            };
            match api::tag::rename_tag(&vault_path, &dto).await {
                Ok(()) => {
                    editing.set(false);
                    on_changed.run(());
                }
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            busy.set(false);
        });
    };

    let ask_delete = move |_: web_sys::MouseEvent| {
        error.set(None);
        confirming.set(true);
    };
    let cancel_delete = move |_: web_sys::MouseEvent| confirming.set(false);

    let confirm_delete = move |_: web_sys::MouseEvent| {
        if busy.get() {
            return;
        }
        let vault_path = active.path.get().unwrap_or_default();
        let id = tag_id.get_value();
        let err_prefix = t_string!(i18n, vault.err_tag_delete).to_string();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match api::tag::delete_tag(&vault_path, &id).await {
                // The row disappears when the refreshed catalog omits it.
                Ok(()) => on_changed.run(()),
                Err(e) => error.set(Some(format!("{err_prefix}{e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-1 py-2">
            {move || {
                if editing.get() {
                    Either::Left(view! {
                        <div class="flex gap-2 items-center">
                            <div class="flex-1">
                                <Input
                                    id="tag-manager-rename"
                                    value=Signal::derive(move || edit_name.get())
                                    on_input=Callback::new(move |v: String| edit_name.set(v))
                                />
                            </div>
                            <Button variant=Variant::Primary size=Size::Sm on:click=save_rename>
                                {move || t!(i18n, vault.save)}
                            </Button>
                            <Button variant=Variant::Ghost size=Size::Sm on:click=cancel_rename>
                                {move || t!(i18n, vault.cancel)}
                            </Button>
                        </div>
                    })
                } else {
                    let name = tag_name.clone();
                    Either::Right(view! {
                        <div class="flex gap-2 items-center justify-between">
                            <span class="text-sm text-text-primary truncate">{name}</span>
                            <Show
                                when=move || confirming.get()
                                fallback=move || view! {
                                    <div class="flex gap-1 shrink-0">
                                        <Button
                                            variant=Variant::Ghost
                                            size=Size::Sm
                                            on:click=start_rename
                                        >
                                            {move || t!(i18n, vault.tag_rename)}
                                        </Button>
                                        <Button
                                            variant=Variant::Danger
                                            size=Size::Sm
                                            on:click=ask_delete
                                        >
                                            {move || t!(i18n, vault.tag_delete)}
                                        </Button>
                                    </div>
                                }
                            >
                                <div class="flex gap-1 items-center shrink-0">
                                    <span class="text-xs text-foreground/60">
                                        {move || t!(i18n, vault.tag_delete_confirm)}
                                    </span>
                                    <Button
                                        variant=Variant::Danger
                                        size=Size::Sm
                                        on:click=confirm_delete
                                    >
                                        {move || t!(i18n, vault.tag_delete)}
                                    </Button>
                                    <Button
                                        variant=Variant::Ghost
                                        size=Size::Sm
                                        on:click=cancel_delete
                                    >
                                        {move || t!(i18n, vault.cancel)}
                                    </Button>
                                </div>
                            </Show>
                        </div>
                    })
                }
            }}
            {move || {
                error
                    .get()
                    .map(|e| view! {
                        <p class="text-xs" style="color:var(--color-danger-text)">{e}</p>
                    })
            }}
        </div>
    }
}
