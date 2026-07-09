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
use crate::i18n::{t, t_string, use_i18n};
use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{CreateTagDto, IndexEntryDto, RenameTagDto, TagMetaDto};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

#[component]
pub fn TagManager(
    open: RwSignal<bool>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
    /// Loaded entries — used to warn how many reference a tag before deleting it.
    #[prop(into)]
    entries: Signal<Vec<IndexEntryDto>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let new_name = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

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
        let err_prefix = t_string!(i18n, vault.err_tag_create).to_owned();
        busy.set(true);
        spawn_local(async move {
            let dto = CreateTagDto { name, color: None };
            match api::tag::create_tag(&vault_path, &dto).await {
                Ok(_id) => {
                    new_name.set(String::new());
                    on_changed.run(());
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            busy.set(false);
        });
    };

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.tag_manager_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                // `.dialog__body` has no bottom padding, so add our own so the
                // list doesn't run to the dialog edge; min-width keeps the Sm
                // dialog from feeling cramped.
                <div class="flex flex-col gap-4 pb-6 min-w-[20rem]">
                    <div class="flex gap-2 items-center">
                        <div class="flex-1">
                            <Input
                                id="tag-manager-new"
                                value=Signal::derive(move || new_name.get())
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, vault.tag_new).to_owned()
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

                    <div class="flex flex-col divide-y divide-secondary/15 max-h-[45vh] overflow-auto">
                        <For
                            each=move || tags.get()
                            // Key on (id, name) so a rename (same id, new name)
                            // re-creates the row — a keyed `<For>` otherwise keeps
                            // the stale child for a fixed key.
                            key=|t| (t.id.clone(), t.name.clone())
                            children=move |t| {
                                view! {
                                    <TagManagerRow tag=t entries=entries on_changed=on_changed />
                                }
                            }
                        />
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

#[component]
fn TagManagerRow(
    tag: TagMetaDto,
    entries: Signal<Vec<IndexEntryDto>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let tag_id = StoredValue::new(tag.id.clone());
    // How many loaded entries reference this tag (shown in the delete warning).
    let ref_count = move || {
        let id = tag_id.get_value();
        entries
            .get()
            .iter()
            .filter(|e| e.tag_ids.iter().any(|t| t == &id))
            .count()
    };
    let tag_name = tag.name.clone();
    // `Copy` handle to the name so `start_rename` stays a `Copy` closure (it's
    // used inside a re-runnable view closure, which can't move a `String` out).
    let name_sv = StoredValue::new(tag.name.clone());

    let editing = RwSignal::new(false);
    let confirming = RwSignal::new(false);
    let edit_name = RwSignal::new(tag.name);
    let busy = RwSignal::new(false);

    let start_rename = move |_: web_sys::MouseEvent| {
        edit_name.set(name_sv.get_value());
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
        let err_prefix = t_string!(i18n, vault.err_tag_rename).to_owned();
        busy.set(true);
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
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            busy.set(false);
        });
    };

    let ask_delete = move |_: web_sys::MouseEvent| {
        confirming.set(true);
    };
    let cancel_delete = move |_: web_sys::MouseEvent| confirming.set(false);

    let confirm_delete = move |_: web_sys::MouseEvent| {
        if busy.get() {
            return;
        }
        let vault_path = active.path.get().unwrap_or_default();
        let id = tag_id.get_value();
        let err_prefix = t_string!(i18n, vault.err_tag_delete).to_owned();
        busy.set(true);
        spawn_local(async move {
            match api::tag::delete_tag(&vault_path, &id).await {
                // The row disappears when the refreshed catalog omits it.
                Ok(()) => on_changed.run(()),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex flex-col gap-1 py-2">
            {move || {
                if editing.get() {
                    Either::Left(
                        view! {
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
                        },
                    )
                } else {
                    let name = tag_name.clone();
                    Either::Right(
                        view! {
                            <div class="flex flex-col gap-1.5">
                                <div class="flex gap-2 items-center justify-between">
                                    <span class="text-sm text-text-primary truncate">{name}</span>
                                    <Show when=move || !confirming.get()>
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
                                    </Show>
                                </div>
                                // Delete confirm: a stacked warning that reports how
                                // many entries still reference the tag (deletion is
                                // allowed — it just leaves tolerated dangling ids).
                                <Show when=move || confirming.get()>
                                    <div class="flex flex-col gap-1.5 rounded-md bg-primary/5 p-2">
                                        <span
                                            class="text-xs"
                                            style="color:var(--color-danger-text)"
                                        >
                                            {move || {
                                                let n = ref_count();
                                                t!(i18n, vault.tag_delete_confirm, refs = n)
                                            }}
                                        </span>
                                        <div class="flex gap-1 justify-end">
                                            <Button
                                                variant=Variant::Ghost
                                                size=Size::Sm
                                                on:click=cancel_delete
                                            >
                                                {move || t!(i18n, vault.cancel)}
                                            </Button>
                                            <Button
                                                variant=Variant::Danger
                                                size=Size::Sm
                                                on:click=confirm_delete
                                            >
                                                {move || t!(i18n, vault.tag_delete)}
                                            </Button>
                                        </div>
                                    </div>
                                </Show>
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}
