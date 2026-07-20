//! Destructive-action confirms for the Trash view (slice 3.6). Both mirror the
//! [`FolderDelete`](super::folder_delete::FolderDelete) pattern: an always-mounted
//! dialog that self-gates on its trigger signal, with a Ghost cancel + a Danger
//! confirm. Restore is non-destructive and needs no dialog.
//!
//! The actual deletion (routing through `hard_delete_entry` — never soft, or the
//! Document blob sidecar leaks) is the parent's `on_confirm`; these components only
//! gather intent.

use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use vedge_ipc::IndexEntryDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogFooter, DialogHeader, DialogTitle};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

/// Confirm the permanent deletion of a single trashed entry. Open iff `target`
/// is `Some`; `on_confirm` receives the entry id.
#[component]
pub fn PermanentDeleteDialog(
    target: RwSignal<Option<IndexEntryDto>>,
    on_confirm: Callback<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let close = Callback::new(move |()| target.set(None));

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.hard_delete_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    {move || {
                        target
                            .get()
                            .map(|e| {
                                view! {
                                    <p class="text-sm font-medium text-text-primary truncate">
                                        {e.name}
                                    </p>
                                }
                            })
                    }}
                    <p class="text-sm text-foreground/70">
                        {move || t!(i18n, vault.hard_delete_confirm)}
                    </p>
                </div>
            </DialogBody>
            <DialogFooter>
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
                    attr:data-testid="confirm-delete-permanent"
                    on:click=move |_: web_sys::MouseEvent| {
                        if let Some(e) = target.get_untracked() {
                            on_confirm.run(e.id);
                        }
                        close.run(());
                    }
                >
                    {move || t!(i18n, vault.delete_permanently)}
                </Button>
            </DialogFooter>
        </Dialog>
    }
}

/// Confirm emptying the whole trash. Open iff `open` is `true`; `count` is the
/// number of trashed items (interpolated into the warning); `on_confirm` fires
/// the bulk hard-delete.
#[component]
pub fn EmptyTrashDialog(
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
                <DialogTitle>{move || t!(i18n, vault.empty_trash_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[20rem]">
                    <p class="text-sm text-foreground/70">
                        {move || {
                            let n = count.get();
                            t!(i18n, vault.empty_trash_confirm, count = n)
                        }}
                    </p>
                </div>
            </DialogBody>
            <DialogFooter>
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
                    attr:data-testid="confirm-empty-trash"
                    on:click=move |_: web_sys::MouseEvent| {
                        on_confirm.run(());
                        close.run(());
                    }
                >
                    {move || t!(i18n, vault.empty_trash_action)}
                </Button>
            </DialogFooter>
        </Dialog>
    }
}
