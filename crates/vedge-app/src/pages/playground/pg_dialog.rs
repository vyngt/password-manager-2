use leptos::prelude::*;
use vedge_ui::components::Input;
use vedge_ui::components::button::Button;
use vedge_ui::components::feedback::dialog::{
    Dialog, DialogBody, DialogFooter, DialogHeader, DialogTitle,
};
use vedge_ui::components::form::label::Label;
use vedge_ui::primitives::tokens::{DialogSize, Variant};

use super::common::Section;

#[derive(Clone, Copy, PartialEq)]
enum ActiveDialog {
    None,
    Delete(u32),
    Rename(u32),
    Export,
}

#[component]
pub fn DialogPage() -> impl IntoView {
    // --- Section 1: Sizes ---
    let active_size = RwSignal::new(Option::<DialogSize>::None);

    let open_size = move |s: DialogSize| {
        active_size.set(Some(s));
    };
    let close_size = Callback::new(move |()| active_size.set(None));

    // --- Section 2: Closeable vs non-closeable ---
    let closeable_open = RwSignal::new(false);
    let non_closeable_open = RwSignal::new(false);

    // --- Section 3: Form ---
    let form_open = RwSignal::new(false);
    let name_val = RwSignal::new(String::new());

    // --- Section 4: Destructive ---
    let destructive_open = RwSignal::new(false);
    let deleted_count = RwSignal::new(0u32);

    // --- Section 5: Multiple dialogs (enum pattern) ---
    let dialog = RwSignal::new(ActiveDialog::None);
    let close_dialog = Callback::new(move |()| dialog.set(ActiveDialog::None));

    let delete_open = Signal::derive(move || matches!(dialog.get(), ActiveDialog::Delete(_)));
    let rename_open = Signal::derive(move || matches!(dialog.get(), ActiveDialog::Rename(_)));
    let export_open = Signal::derive(move || matches!(dialog.get(), ActiveDialog::Export));

    let delete_target_id = Signal::derive(move || match dialog.get() {
        ActiveDialog::Delete(id) => Some(id),
        _ => None,
    });
    let rename_target_id = Signal::derive(move || match dialog.get() {
        ActiveDialog::Rename(id) => Some(id),
        _ => None,
    });

    // --- Section 6: Body-only ---
    let body_only_open = RwSignal::new(false);

    // --- Section 7: Flush body (edge-to-edge list) ---
    let flush_open = RwSignal::new(false);

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Dialog"</h1>

            // --- Sizes ---
            <Section title="Sizes">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Secondary on:click=move |_| open_size(DialogSize::Sm)>
                        "Small (400px)"
                    </Button>
                    <Button variant=Variant::Secondary on:click=move |_| open_size(DialogSize::Md)>
                        "Medium (560px)"
                    </Button>
                    <Button variant=Variant::Secondary on:click=move |_| open_size(DialogSize::Lg)>
                        "Large (720px)"
                    </Button>
                    <Button
                        variant=Variant::Secondary
                        on:click=move |_| open_size(DialogSize::Full)
                    >
                        "Full (viewport)"
                    </Button>
                </div>

                <Dialog
                    open=Signal::derive(move || active_size.get() == Some(DialogSize::Sm))
                    on_close=close_size
                    size=DialogSize::Sm
                >
                    <DialogHeader>
                        <DialogTitle>"Small dialog"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "max-width: 400px — simple confirmations, short messages."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Primary on:click=move |_| close_size.run(())>
                            "Close"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog
                    open=Signal::derive(move || active_size.get() == Some(DialogSize::Md))
                    on_close=close_size
                    size=DialogSize::Md
                >
                    <DialogHeader>
                        <DialogTitle>"Medium dialog"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "max-width: 560px — the default. Short forms, rename, quick edits."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Primary on:click=move |_| close_size.run(())>
                            "Close"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog
                    open=Signal::derive(move || active_size.get() == Some(DialogSize::Lg))
                    on_close=close_size
                    size=DialogSize::Lg
                >
                    <DialogHeader>
                        <DialogTitle>"Large dialog"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">"max-width: 720px — complex forms, multi-field entry."</p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Primary on:click=move |_| close_size.run(())>
                            "Close"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog
                    open=Signal::derive(move || active_size.get() == Some(DialogSize::Full))
                    on_close=close_size
                    size=DialogSize::Full
                >
                    <DialogHeader>
                        <DialogTitle>"Full-width dialog"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "max-width: 100vw - 32px — immersive content (wizards, workspaces)."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Primary on:click=move |_| close_size.run(())>
                            "Close"
                        </Button>
                    </DialogFooter>
                </Dialog>
            </Section>

            // --- Closeable / Non-closeable ---
            <Section title="Closeable vs. non-closeable">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Secondary on:click=move |_| closeable_open.set(true)>
                        "Open closeable"
                    </Button>
                    <Button
                        variant=Variant::Secondary
                        on:click=move |_| non_closeable_open.set(true)
                    >
                        "Open non-closeable"
                    </Button>
                </div>
                <p class="text-xs text-text-tertiary">
                    "Closeable: Escape, backdrop click, or × button all dismiss. Non-closeable: only the confirm button."
                </p>

                <Dialog
                    open=closeable_open
                    on_close=Callback::new(move |()| closeable_open.set(false))
                    size=DialogSize::Sm
                >
                    <DialogHeader>
                        <DialogTitle>"Closeable"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">"Press Escape, click outside, or use the × button."</p>
                    </DialogBody>
                    <DialogFooter>
                        <Button
                            variant=Variant::Primary
                            on:click=move |_| closeable_open.set(false)
                        >
                            "OK"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog
                    open=non_closeable_open
                    on_close=Callback::new(move |()| non_closeable_open.set(false))
                    closeable=false
                    size=DialogSize::Sm
                >
                    <DialogHeader>
                        <DialogTitle>"Critical confirmation"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "This dialog cannot be dismissed by Escape or backdrop click. Use the button below to proceed."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button
                            variant=Variant::Primary
                            on:click=move |_| non_closeable_open.set(false)
                        >
                            "Acknowledge"
                        </Button>
                    </DialogFooter>
                </Dialog>
            </Section>

            // --- Form ---
            <Section title="Form content">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Secondary on:click=move |_| form_open.set(true)>
                        "Rename entry"
                    </Button>
                    <span class="text-sm text-text-secondary self-center">
                        "Current: "
                        <span class="font-medium text-text-primary">
                            {move || {
                                let v = name_val.get();
                                if v.is_empty() { "(unset)".to_owned() } else { v }
                            }}
                        </span>
                    </span>
                </div>

                <Dialog
                    open=form_open
                    on_close=Callback::new(move |()| form_open.set(false))
                    size=DialogSize::Md
                >
                    <DialogHeader>
                        <DialogTitle>"Rename entry"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <div class="py-2 space-y-2">
                            <Label html_for="rename-name">"Name"</Label>
                            <Input
                                id="rename-name"
                                value=Signal::derive(move || name_val.get())
                                on_input=Callback::new(move |v: String| name_val.set(v))
                                placeholder=Signal::stored("Enter a new name".to_owned())
                            />
                        </div>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Secondary on:click=move |_| form_open.set(false)>
                            "Cancel"
                        </Button>
                        <Button variant=Variant::Primary on:click=move |_| form_open.set(false)>
                            "Save"
                        </Button>
                    </DialogFooter>
                </Dialog>
            </Section>

            // --- Destructive ---
            <Section title="Destructive action">
                <div class="flex flex-wrap gap-2 items-center">
                    <Button variant=Variant::Danger on:click=move |_| destructive_open.set(true)>
                        "Delete entry…"
                    </Button>
                    <span class="text-sm text-text-secondary">
                        "Deleted so far: "
                        <span class="font-medium text-text-primary">
                            {move || deleted_count.get().to_string()}
                        </span>
                    </span>
                </div>

                <Dialog
                    open=destructive_open
                    on_close=Callback::new(move |()| destructive_open.set(false))
                    size=DialogSize::Sm
                >
                    <DialogHeader>
                        <DialogTitle>"Delete entry?"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "This action cannot be undone. The entry will be permanently removed from your vault."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button
                            variant=Variant::Secondary
                            on:click=move |_| destructive_open.set(false)
                        >
                            "Cancel"
                        </Button>
                        <Button
                            variant=Variant::Danger
                            attr:style="margin-left: auto"
                            on:click=move |_| {
                                deleted_count.update(|n| *n += 1);
                                destructive_open.set(false);
                            }
                        >
                            "Delete"
                        </Button>
                    </DialogFooter>
                </Dialog>
            </Section>

            // --- Multiple dialogs via enum ---
            <Section title="Multiple dialogs (enum pattern)">
                <p class="text-xs text-text-tertiary">
                    "One signal per screen — mutually exclusive by construction. Only one Dialog can ever be open."
                </p>
                <div class="flex flex-wrap gap-2">
                    <Button
                        variant=Variant::Secondary
                        on:click=move |_| dialog.set(ActiveDialog::Delete(42))
                    >
                        "Delete entry #42"
                    </Button>
                    <Button
                        variant=Variant::Secondary
                        on:click=move |_| dialog.set(ActiveDialog::Rename(42))
                    >
                        "Rename entry #42"
                    </Button>
                    <Button
                        variant=Variant::Secondary
                        on:click=move |_| dialog.set(ActiveDialog::Export)
                    >
                        "Export vault"
                    </Button>
                </div>

                <Dialog open=delete_open on_close=close_dialog size=DialogSize::Sm>
                    <DialogHeader>
                        <DialogTitle>"Delete entry"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "Delete entry #"
                            {move || {
                                delete_target_id.get().map(|id| id.to_string()).unwrap_or_default()
                            }} "? This action cannot be undone."
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Secondary on:click=move |_| close_dialog.run(())>
                            "Cancel"
                        </Button>
                        <Button
                            variant=Variant::Danger
                            attr:style="margin-left: auto"
                            on:click=move |_| close_dialog.run(())
                        >
                            "Delete"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog open=rename_open on_close=close_dialog size=DialogSize::Md>
                    <DialogHeader>
                        <DialogTitle>"Rename entry"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">
                            "Renaming entry #"
                            {move || {
                                rename_target_id.get().map(|id| id.to_string()).unwrap_or_default()
                            }}
                        </p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Secondary on:click=move |_| close_dialog.run(())>
                            "Cancel"
                        </Button>
                        <Button variant=Variant::Primary on:click=move |_| close_dialog.run(())>
                            "Save"
                        </Button>
                    </DialogFooter>
                </Dialog>

                <Dialog open=export_open on_close=close_dialog size=DialogSize::Lg>
                    <DialogHeader>
                        <DialogTitle>"Export vault"</DialogTitle>
                    </DialogHeader>
                    <DialogBody>
                        <p class="py-2">"Export configuration would go here."</p>
                    </DialogBody>
                    <DialogFooter>
                        <Button variant=Variant::Secondary on:click=move |_| close_dialog.run(())>
                            "Cancel"
                        </Button>
                        <Button variant=Variant::Primary on:click=move |_| close_dialog.run(())>
                            "Export"
                        </Button>
                    </DialogFooter>
                </Dialog>
            </Section>

            // --- Body-only ---
            <Section title="Body-only">
                <p class="text-xs text-text-tertiary">
                    "No header, no footer — the dialog container itself receives focus since there are no focusable elements inside."
                </p>
                <Button variant=Variant::Secondary on:click=move |_| body_only_open.set(true)>
                    "Open body-only"
                </Button>

                <Dialog
                    open=body_only_open
                    on_close=Callback::new(move |()| body_only_open.set(false))
                    size=DialogSize::Sm
                >
                    <DialogBody>
                        <div class="py-6 text-center">
                            <p>"Click outside or press Escape to close."</p>
                        </div>
                    </DialogBody>
                </Dialog>
            </Section>

            // --- Flush body (edge-to-edge list) ---
            <Section title="Flush body">
                <p class="text-xs text-text-tertiary">
                    "`flush` drops the body padding so list rows run to the border. The default (padded) is the safe case — a list dialog opts in."
                </p>
                <Button variant=Variant::Secondary on:click=move |_| flush_open.set(true)>
                    "Open flush list"
                </Button>

                <Dialog
                    open=flush_open
                    on_close=Callback::new(move |()| flush_open.set(false))
                    size=DialogSize::Sm
                >
                    <DialogHeader>
                        <DialogTitle>"Pick an item"</DialogTitle>
                    </DialogHeader>
                    <DialogBody flush=true>
                        <div class="flex flex-col divide-y divide-border">
                            {(1..=6)
                                .map(|n| {
                                    view! {
                                        <button
                                            type="button"
                                            class="text-left px-6 py-2.5 hover:bg-surface-2"
                                            on:click=move |_| flush_open.set(false)
                                        >
                                            {format!("Item {n}")}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </DialogBody>
                </Dialog>
            </Section>
        </div>
    }
}
