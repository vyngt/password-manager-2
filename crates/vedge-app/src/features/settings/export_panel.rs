//! Export panel (slice 5.3a) — the Settings → Export tab.
//!
//! Two formats behind one action: an **encrypted** envelope (all entry types,
//! sealed with a passphrase) or a **plaintext** logins-only CSV. The honest ⑧
//! warning is always visible, and the CSV path carries an extra, louder one — a
//! plaintext file cannot be reliably wiped, and `VEdge` never pretends otherwise.
//!
//! Every `t_string!` read from a `spawn_local` future is `untrack`ed / hoisted
//! before the async block, per the reactive-owner rule.

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::ExportReportDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn ExportPanel() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let busy = RwSignal::new(false);
    let encrypted = RwSignal::new(true);
    let passphrase = RwSignal::new(String::new());
    let spreadsheet_safe = RwSignal::new(false);
    let last = RwSignal::new(None::<ExportReportDto>);

    let show = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
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
                    match api::export::export(&path, &dest, enc, pass_ref, safe, None).await {
                        Ok(report) => {
                            last.set(Some(report));
                            show(saved, ToastVariant::Success);
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
        // ---- Intro ----
        <div class="py-3.5 border-b border-border">
            <div class="text-sm font-medium text-text-primary">
                {move || t!(i18n, settings.export_title)}
            </div>
            <div class="text-xs text-text-secondary mt-0.5">
                {move || t!(i18n, settings.export_desc)}
            </div>
        </div>

        // ---- 🔴 Decision ⑧ — the honest warning, always on. ----
        <div
            class="py-3 text-xs font-medium border-b border-border"
            style="color:var(--color-warning-text)"
            data-testid="export-warning"
        >
            {move || t!(i18n, settings.export_warning)}
        </div>

        // ---- Format: encrypt toggle ----
        <div class="py-3.5 border-b border-border space-y-2">
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
                            <div class="text-xs font-medium" style="color:var(--color-danger-text)">
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
                        placeholder=move || t_string!(i18n, settings.export_passphrase_placeholder)
                        prop:value=move || passphrase.get()
                        on:input:target=move |ev| passphrase.set(ev.target().value())
                        data-testid="export-passphrase"
                    />
                </div>
            </Show>
        </div>

        // ---- Export action ----
        <div class="flex items-center justify-end py-3.5 border-b border-border">
            {move || {
                let b = busy.get();
                let disabled = b || (encrypted.get() && passphrase.get().trim().is_empty());
                view! {
                    <Button
                        variant=Variant::Primary
                        loading=b
                        disabled=disabled
                        attr:data-testid="export-now"
                        on:click=move |_: web_sys::MouseEvent| do_export()
                    >
                        {move || t!(i18n, settings.export_button)}
                    </Button>
                }
            }}
        </div>

        // ---- Last-export report ----
        {move || {
            last.get()
                .map(|r| {
                    view! {
                        <div class="py-3.5 text-xs text-text-secondary" data-testid="export-report">
                            <span class="font-medium text-text-primary">
                                {move || t!(i18n, settings.export_report_title)}
                            </span>
                            ": "
                            {r.entry_count.to_string()}
                            " "
                            {move || t!(i18n, settings.export_report_entries)}
                        </div>
                    }
                })
        }}
    }
}
