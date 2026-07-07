//! Folder-delete confirmation. An empty folder is deleted directly by the page;
//! a **non-empty** folder opens this two-stage dialog instead: it reports how many
//! items the folder holds and offers **Empty folder** (a recursive soft-delete of
//! every descendant entry + sub-folder via [`subtree_contents`]). Because the
//! content count is read reactively from `items`, once the recursive empty lands
//! the dialog flips to **Delete folder** (now a safe delete) — "empty first, then
//! delete", in one dialog.

use super::folder_tree::{FolderNode, subtree_contents};
use crate::i18n::*;
use leptos::either::Either;
use leptos::prelude::*;
use vedge_ipc::IndexEntryDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

#[component]
pub fn FolderDelete(
    target: RwSignal<Option<IndexEntryDto>>,
    #[prop(into)] items: Signal<Vec<IndexEntryDto>>,
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    /// Recursively soft-delete these ids (the folder's contents).
    on_empty: Callback<Vec<String>>,
    /// Soft-delete the now-empty folder itself.
    on_delete_folder: Callback<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let close = Callback::new(move |()| target.set(None));

    // Ids inside the target folder, recomputed reactively so the dialog advances
    // from "empty me" to "delete me" after the recursive empty refreshes `items`.
    let contents = move || -> Vec<String> {
        target
            .get()
            .map(|e| subtree_contents(&items.get(), &folders.get(), &e.id))
            .unwrap_or_default()
    };

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.folder_delete_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-6 min-w-[20rem]">
                    {move || {
                        target
                            .get()
                            .map(|e| {
                                view! {
                                    <p class="text-sm font-medium text-text-primary truncate">{e.name}</p>
                                }
                            })
                    }}
                    {move || {
                        if contents().is_empty() {
                            Either::Left(
                                view! {
                                    <p class="text-sm text-foreground/70">
                                        {move || t!(i18n, vault.folder_delete_empty_confirm)}
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
                                            on:click=move |_: web_sys::MouseEvent| {
                                                if let Some(e) = target.get_untracked() {
                                                    on_delete_folder.run(e.id);
                                                }
                                            }
                                        >
                                            {move || t!(i18n, vault.folder_delete)}
                                        </Button>
                                    </div>
                                },
                            )
                        } else {
                            Either::Right(
                                view! {
                                    <p class="text-sm text-foreground/70">
                                        {move || {
                                            let n = contents().len();
                                            t!(i18n, vault.folder_delete_not_empty, count = n)
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
                                            on:click=move |_: web_sys::MouseEvent| on_empty.run(contents())
                                        >
                                            {move || t!(i18n, vault.folder_empty_action)}
                                        </Button>
                                    </div>
                                },
                            )
                        }
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}
