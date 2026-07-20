//! Dialogs for the bulk action bar (slice 5.3.1c): the Trash confirm, the
//! move-to-folder picker, and the add-tags picker. Each gathers intent for a
//! *set* of selected entries; the parent (`vault.rs`) loops the single-entry
//! operation. They mirror the existing single-entry dialogs
//! ([`super::trash_confirm`], [`super::folder_move`], [`super::tag_assign`]).

use super::folder_tree::{FlatFolder, FolderNode, flatten_tree};
use super::tag_assign::toggle_tag;
use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use std::collections::HashSet;
use vedge_ipc::TagMetaDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

/// Indent a flattened folder's name for a `Select` option label (options can't
/// carry padding, so pad the text with figure spaces). Mirrors the private
/// helper in [`super::folder_move`].
fn indent_label(f: &FlatFolder) -> String {
    let pad = "\u{2007}".repeat(f.depth.saturating_mul(2));
    format!("{pad}{}", f.name)
}

/// Confirm moving the N selected entries to Trash. Soft-delete is reversible, but
/// a destructive verb on a *set* is higher-stakes than the single-row action, so
/// it confirms (mirrors [`super::trash_confirm::EmptyTrashDialog`]).
#[component]
pub fn BulkTrashDialog(
    open: RwSignal<bool>,
    #[prop(into)] count: Signal<usize>,
    on_confirm: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let close = Callback::new(move |()| open.set(false));

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.bulk_trash_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    <p class="text-sm text-foreground/70">
                        {move || {
                            let n = count.get();
                            t!(i18n, vault.bulk_trash_message, count = n)
                        }}
                    </p>
                    <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        <Button
                            variant=Variant::Danger
                            size=Size::Sm
                            attr:data-testid="confirm-bulk-trash"
                            on:click=move |_: web_sys::MouseEvent| {
                                on_confirm.run(());
                                close.run(());
                            }
                        >
                            {move || t!(i18n, vault.bulk_trash_action)}
                        </Button>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// Move the N selected entries to one destination folder (or root). None of the
/// rows are folders (folders aren't listed), so every folder is a valid target —
/// no self/descendant exclusion is needed. Fires `on_move(destination)`.
#[component]
pub fn SelectionMoveDialog(
    open: RwSignal<bool>,
    #[prop(into)] count: Signal<usize>,
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    on_move: Callback<Option<String>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chosen = RwSignal::new(Option::<String>::None);

    // Reset the picker to root each time it opens.
    Effect::new(move |_| {
        if open.get() {
            chosen.set(None);
        }
    });

    let close = Callback::new(move |()| open.set(false));
    let confirm = move |_: web_sys::MouseEvent| {
        on_move.run(chosen.get_untracked());
        open.set(false);
    };

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>
                    {move || {
                        let n = count.get();
                        t!(i18n, vault.bulk_move_title, count = n)
                    }}
                </DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    {move || {
                        let mut options = vec![
                            SelectItem::option("", t_string!(i18n, vault.folder_none).to_owned()),
                        ];
                        options
                            .extend(
                                flatten_tree(&folders.get(), &HashSet::new())
                                    .iter()
                                    .map(|f| SelectItem::option(f.id.clone(), indent_label(f))),
                            );
                        view! {
                            <Select
                                options=options
                                value=Signal::derive(move || chosen.get().unwrap_or_default())
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, vault.folder_move_dest_aria).to_owned()
                                })
                                on_change=Callback::new(move |v: String| {
                                    chosen.set(if v.is_empty() { None } else { Some(v) });
                                })
                            />
                        }
                    }} <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        <Button
                            variant=Variant::Primary
                            size=Size::Sm
                            attr:data-testid="confirm-bulk-move"
                            on:click=confirm
                        >
                            {move || t!(i18n, vault.folder_move_here)}
                        </Button>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// Add tags to the N selected entries. **Add-only** (union) — removing a tag
/// across a heterogeneous selection is ambiguous, so the picker only chooses tags
/// to add; the page unions them into each entry's existing set. Fires
/// `on_apply(chosen_tag_ids)`.
#[component]
pub fn SelectionTagDialog(
    open: RwSignal<bool>,
    #[prop(into)] count: Signal<usize>,
    #[prop(into)] tags: Signal<Vec<TagMetaDto>>,
    on_apply: Callback<Vec<String>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chosen = RwSignal::new(Vec::<String>::new());

    // Reset the picks each time it opens.
    Effect::new(move |_| {
        if open.get() {
            chosen.set(Vec::new());
        }
    });

    let close = Callback::new(move |()| open.set(false));

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>
                    {move || {
                        let n = count.get();
                        t!(i18n, vault.bulk_tag_title, count = n)
                    }}
                </DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    <Show
                        when=move || !tags.get().is_empty()
                        fallback=move || {
                            view! {
                                <p class="text-sm text-foreground/60">
                                    {move || t!(i18n, vault.bulk_tag_none)}
                                </p>
                            }
                        }
                    >
                        <div class="flex flex-wrap gap-2">
                            {move || {
                                tags.get()
                                    .into_iter()
                                    .map(|tag| {
                                        let id = tag.id;
                                        let name = tag.name;
                                        let cls_id = id.clone();
                                        let aria_id = id.clone();
                                        view! {
                                            <button
                                                type="button"
                                                class=move || {
                                                    if chosen.with(|c| c.contains(&cls_id)) {
                                                        "inline-flex items-center rounded-full border px-2.5 py-1 text-xs transition-colors border-primary bg-primary/10 text-primary"
                                                    } else {
                                                        "inline-flex items-center rounded-full border px-2.5 py-1 text-xs transition-colors border-border text-foreground/70 hover:border-primary/60"
                                                    }
                                                }
                                                aria-pressed=move || {
                                                    chosen.with(|c| c.contains(&aria_id)).then_some("true")
                                                }
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    chosen.update(|c| *c = toggle_tag(c, &id));
                                                }
                                            >
                                                {name}
                                            </button>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </Show>
                    <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            size=Size::Sm
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        {move || {
                            let disabled = chosen.with(Vec::is_empty);
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    size=Size::Sm
                                    disabled=disabled
                                    attr:data-testid="confirm-bulk-tag"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        let picks = chosen.get_untracked();
                                        if !picks.is_empty() {
                                            on_apply.run(picks);
                                        }
                                        open.set(false);
                                    }
                                >
                                    {move || t!(i18n, vault.bulk_tag_apply)}
                                </Button>
                            }
                        }}
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
