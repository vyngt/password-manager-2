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
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::radio_group::{RadioGroup, RadioOption};
use vedge_ui::components::{Button, Checkbox, Input};
use vedge_ui::primitives::tokens::{DialogSize, Orientation, ToastVariant, Variant};

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
                <div class="flex flex-col gap-4 min-w-[22rem]">
                    // ---- Scope ----
                    <div class="flex flex-col gap-1.5">
                        <span class="text-foreground/50 text-xs uppercase tracking-wider">
                            {move || t!(i18n, vault.export_scope_heading)}
                        </span>
                        // Built in a reactive closure so the labels track the locale
                        // (relocalize on switch) AND the live counts. The modal freezes
                        // the underlying set, so the options are stable while it is open.
                        {move || {
                            let mut options = Vec::new();
                            if !selected.get().is_empty() {
                                options
                                    .push(
                                        RadioOption::new(
                                            "selection",
                                            format!(
                                                "{} ({})",
                                                t_string!(i18n, vault.export_scope_selection),
                                                selected.get().len(),
                                            ),
                                        ),
                                    );
                            }
                            options
                                .push(
                                    RadioOption::new(
                                        "filter",
                                        format!(
                                            "{} ({})",
                                            t_string!(i18n, vault.export_scope_filter),
                                            filtered.get().len(),
                                        ),
                                    ),
                                );
                            options
                                .push(
                                    RadioOption::new(
                                        "all",
                                        format!(
                                            "{} ({})",
                                            t_string!(i18n, vault.export_scope_all),
                                            total.get(),
                                        ),
                                    ),
                                );
                            view! {
                                <RadioGroup
                                    options=options
                                    orientation=Orientation::Vertical
                                    value=Signal::derive(move || {
                                        scope_to_str(scope.get()).to_owned()
                                    })
                                    on_change=Callback::new(move |v: String| {
                                        scope.set(scope_from_str(&v));
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.export_scope_heading).to_owned()
                                    })
                                />
                            }
                        }}
                    </div>

                    // ---- 🔴 ⑧ the honest warning, always on ----
                    <div class="text-xs font-medium" style="color:var(--color-warning-text)">
                        {move || t!(i18n, settings.export_warning)}
                    </div>

                    // ---- Format ----
                    <div class="flex items-start gap-2">
                        <Checkbox
                            checked=Signal::derive(move || encrypted.get())
                            on_change=Callback::new(move |v: bool| encrypted.set(v))
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, settings.export_encrypt_label).to_owned()
                            })
                            class="mt-0.5"
                        />
                        <span>
                            <span class="text-sm font-medium text-text-primary">
                                {move || t!(i18n, settings.export_encrypt_label)}
                            </span>
                            <span class="block text-xs text-text-secondary">
                                {move || t!(i18n, settings.export_encrypt_desc)}
                            </span>
                        </span>
                    </div>
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
                                    <div class="flex items-center gap-2 text-xs text-text-secondary">
                                        <Checkbox
                                            checked=Signal::derive(move || spreadsheet_safe.get())
                                            on_change=Callback::new(move |v: bool| {
                                                spreadsheet_safe.set(v);
                                            })
                                            aria_label=Signal::derive(move || {
                                                t_string!(i18n, settings.export_spreadsheet_safe).to_owned()
                                            })
                                        />
                                        {move || t!(i18n, settings.export_spreadsheet_safe)}
                                    </div>
                                </div>
                            }
                        }
                    >
                        <div class="pl-6">
                            <Input
                                id="export-passphrase"
                                input_type="password"
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, settings.export_passphrase_placeholder)
                                        .to_owned()
                                })
                                value=Signal::derive(move || passphrase.get())
                                on_input=Callback::new(move |v: String| passphrase.set(v))
                                reveal_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.show_password).to_owned()
                                })
                                hide_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.hide_password).to_owned()
                                })
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

/// `ExportScope` <-> the `RadioGroup`'s `String` option value.
fn scope_to_str(s: ExportScope) -> &'static str {
    match s {
        ExportScope::Selection => "selection",
        ExportScope::Filter => "filter",
        ExportScope::All => "all",
    }
}

fn scope_from_str(s: &str) -> ExportScope {
    match s {
        "selection" => ExportScope::Selection,
        "filter" => ExportScope::Filter,
        _ => ExportScope::All,
    }
}
