//! The three-scope export dialog (slice 5.3.1d, decision ③): export the **N
//! selected** / the **N matching the filter** / **all N** entries — each with its
//! live count — through the frozen 5.3 adapter. The format card states CSV's
//! "logins only, plain text" constraint at the moment of choice (⑧), reusing the
//! Settings copy; the honest never-fake-a-shred warning is always on.
//!
//! Opened from two places: the toolbar `⋯` "Export all…" (default `All`) and the
//! selection bar's "Export…" (default `Selection`). The chosen scope resolves to
//! an `Option<Vec<EntryId>>` the backend intersects with the active set.
//!
//! Every `t_string!` read from the `spawn_local` future is `untrack`ed / hoisted
//! before the async block, per the reactive-owner rule.

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::primitives::tokens::{DialogSize, ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

/// Which entries a run exports; also the dialog's open/default signal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    Selection,
    Filter,
    All,
}

#[component]
pub fn ExportDialog(
    /// `Some(default_scope)` opens the dialog with that scope preselected; `None`
    /// is closed.
    open: RwSignal<Option<ExportScope>>,
    /// The selection scope's ids.
    #[prop(into)]
    selected: Signal<Vec<String>>,
    /// The filter scope's ids (the currently visible rows).
    #[prop(into)]
    filtered: Signal<Vec<String>>,
    /// The all-scope count (all active entries).
    #[prop(into)]
    total: Signal<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let scope = RwSignal::new(ExportScope::All);
    let encrypted = RwSignal::new(true);
    let passphrase = RwSignal::new(String::new());
    let spreadsheet_safe = RwSignal::new(false);
    let busy = RwSignal::new(false);

    // Seed the scope + reset the passphrase each time it opens.
    Effect::new(move |_| {
        if let Some(s) = open.get() {
            scope.set(s);
            passphrase.set(String::new());
            busy.set(false);
        }
    });

    let close = Callback::new(move |()| open.set(None));
    let show = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_owned());
        toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
    };

    let do_export = move || {
        if busy.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let enc = encrypted.get_untracked();
        let pass = passphrase.get_untracked();
        let safe = spreadsheet_safe.get_untracked();
        if enc && pass.trim().is_empty() {
            return; // the button is disabled in this state; belt-and-suspenders
        }
        // Resolve the scope to a subset (or `None` = all active).
        let ids: Option<Vec<String>> = match scope.get_untracked() {
            ExportScope::All => None,
            ExportScope::Selection => Some(selected.get_untracked()),
            ExportScope::Filter => Some(filtered.get_untracked()),
        };

        let (default_name, ext, filter_name) = if enc {
            ("vedge-export.vedgex", "vedgex", "VEdge Export")
        } else {
            ("vedge-logins.csv", "csv", "CSV")
        };
        let dialog_title = untrack(|| t_string!(i18n, settings.export_button).to_owned());
        let saved = untrack(|| t_string!(i18n, settings.export_saved).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, settings.export_failed).to_owned());
        busy.set(true);
        spawn_local(async move {
            let opts = SaveDialogOptions {
                title: Some(dialog_title),
                default_path: Some(default_name.to_owned()),
                filters: vec![DialogFilter {
                    name: filter_name.to_owned(),
                    extensions: vec![ext.to_owned()],
                }],
            };
            match api::dialog::save(&opts).await {
                Ok(Some(dest)) => {
                    let pass_ref = if enc { Some(pass.as_str()) } else { None };
                    match api::export::export(&path, &dest, enc, pass_ref, safe, ids.as_deref())
                        .await
                    {
                        Ok(_) => {
                            show(saved, ToastVariant::Success);
                            open.set(None);
                        }
                        Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
                    }
                }
                // Cancelled — silent.
                Ok(None) => {}
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    view! {
        <Dialog
            open=Signal::derive(move || open.get().is_some())
            on_close=close
            size=DialogSize::Md
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, vault.export_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-6 min-w-[22rem]">
                    // ---- Scope ----
                    <div class="flex flex-col gap-1.5">
                        <span class="text-foreground/50 text-xs uppercase tracking-wider">
                            {move || t!(i18n, vault.export_scope_heading)}
                        </span>
                        <div role="radiogroup" class="flex flex-col gap-1">
                            <Show when=move || !selected.get().is_empty()>
                                <ScopeRow
                                    this=ExportScope::Selection
                                    scope=scope
                                    label=Signal::derive(move || {
                                        t_string!(i18n, vault.export_scope_selection).to_owned()
                                    })
                                    count=Signal::derive(move || selected.get().len())
                                />
                            </Show>
                            <ScopeRow
                                this=ExportScope::Filter
                                scope=scope
                                label=Signal::derive(move || {
                                    t_string!(i18n, vault.export_scope_filter).to_owned()
                                })
                                count=Signal::derive(move || filtered.get().len())
                            />
                            <ScopeRow
                                this=ExportScope::All
                                scope=scope
                                label=Signal::derive(move || {
                                    t_string!(i18n, vault.export_scope_all).to_owned()
                                })
                                count=total
                            />
                        </div>
                    </div>

                    // ---- 🔴 ⑧ the honest warning, always on ----
                    <div class="text-xs font-medium" style="color:var(--color-warning-text)">
                        {move || t!(i18n, settings.export_warning)}
                    </div>

                    // ---- Format ----
                    <label class="flex items-start gap-2 cursor-pointer">
                        <input
                            type="checkbox"
                            class="mt-0.5"
                            prop:checked=move || encrypted.get()
                            on:change:target=move |ev| encrypted.set(ev.target().checked())
                        />
                        <span>
                            <span class="text-sm font-medium text-text-primary">
                                {move || t!(i18n, settings.export_encrypt_label)}
                            </span>
                            <span class="block text-xs text-text-secondary">
                                {move || t!(i18n, settings.export_encrypt_desc)}
                            </span>
                        </span>
                    </label>
                    <Show
                        when=move || encrypted.get()
                        fallback=move || {
                            view! {
                                <div class="pl-6 space-y-1.5" data-testid="export-csv-options">
                                    <div
                                        class="text-xs font-medium"
                                        style="color:var(--color-danger-text)"
                                    >
                                        {move || t!(i18n, settings.export_csv_warning)}
                                    </div>
                                    <label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer">
                                        <input
                                            type="checkbox"
                                            prop:checked=move || spreadsheet_safe.get()
                                            on:change:target=move |ev| {
                                                spreadsheet_safe.set(ev.target().checked());
                                            }
                                        />
                                        {move || t!(i18n, settings.export_spreadsheet_safe)}
                                    </label>
                                </div>
                            }
                        }
                    >
                        <div class="pl-6">
                            <input
                                type="password"
                                class="w-full rounded-md border border-border bg-surface px-2.5 py-1.5 text-sm text-text-primary"
                                placeholder=move || {
                                    t_string!(i18n, settings.export_passphrase_placeholder)
                                }
                                prop:value=move || passphrase.get()
                                on:input:target=move |ev| passphrase.set(ev.target().value())
                                data-testid="export-passphrase"
                            />
                        </div>
                    </Show>

                    // ---- Actions ----
                    <div class="flex justify-end gap-2">
                        <Button
                            variant=Variant::Ghost
                            on:click=move |_: web_sys::MouseEvent| close.run(())
                        >
                            {move || t!(i18n, vault.cancel)}
                        </Button>
                        {move || {
                            let b = busy.get();
                            let disabled = b
                                || (encrypted.get() && passphrase.get().trim().is_empty());
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    loading=b
                                    disabled=disabled
                                    attr:data-testid="export-run"
                                    on:click=move |_: web_sys::MouseEvent| do_export()
                                >
                                    {move || t!(i18n, settings.export_button)}
                                </Button>
                            }
                        }}
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// One scope radio row: a native radio + label + count.
#[component]
fn ScopeRow(
    this: ExportScope,
    scope: RwSignal<ExportScope>,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] count: Signal<usize>,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-2 text-sm cursor-pointer py-0.5">
            <input
                type="radio"
                name="export-scope"
                prop:checked=move || scope.get() == this
                on:change=move |_: web_sys::Event| scope.set(this)
            />
            <span class="text-text-primary">{move || label.get()}</span>
            <span class="text-foreground/50">"(" {move || count.get()} ")"</span>
        </label>
    }
}
