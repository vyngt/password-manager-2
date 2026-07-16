//! Import panel (slice 5.3b) — the Settings → Import tab.
//!
//! Bring entries back in from an encrypted `.vedgex` export or a plaintext CSV
//! (a picked file, or **pasted text** — Decision ⑧, the RAM-only no-disk path).
//! The preview crosses **derivatives only** (Decision ⑦): a row shows a `•••`
//! placeholder for a secret, never the value. The user reviews per-row badges,
//! toggles Import/Skip, and only then commits — the write happens entirely in Rust.
//!
//! Every `t_string!` read from a `spawn_local` future is `untrack`ed / hoisted
//! before the async block, per the reactive-owner rule.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{ImportActionDto, ImportPreviewRow, ImportReportDto};
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, OpenDialogOptions};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn ImportPanel() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let busy = RwSignal::new(false);
    let encrypted = RwSignal::new(false);
    let passphrase = RwSignal::new(String::new());
    let src_path = RwSignal::new(String::new());
    let paste_text = RwSignal::new(String::new());
    let preview = RwSignal::new(Vec::<ImportPreviewRow>::new());
    let picked = RwSignal::new(HashMap::<u32, bool>::new());
    let report = RwSignal::new(None::<ImportReportDto>);

    let show = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
    };

    let browse = move || {
        let enc = encrypted.get_untracked();
        let title = untrack(|| t_string!(i18n, settings.import_browse).to_owned());
        spawn_local(async move {
            let (name, ext) = if enc {
                ("VEdge Export", "vedgex")
            } else {
                ("CSV", "csv")
            };
            let opts = OpenDialogOptions {
                title: Some(title),
                filters: vec![DialogFilter {
                    name: name.to_owned(),
                    extensions: vec![ext.to_owned()],
                }],
                directory: false,
            };
            if let Ok(Some(path)) = api::dialog::open(&opts).await {
                src_path.set(path);
            }
        });
    };

    let do_preview = move || {
        if busy.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let enc = encrypted.get_untracked();
        let pass = passphrase.get_untracked();
        let file = src_path.get_untracked();
        let paste = paste_text.get_untracked();
        // Need SOMETHING to read: a file, or (CSV only) pasted text.
        if file.trim().is_empty() && (enc || paste.trim().is_empty()) {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, settings.import_failed).to_owned());
        busy.set(true);
        spawn_local(async move {
            let src = (!file.trim().is_empty()).then_some(file.as_str());
            let text = (!enc && !paste.trim().is_empty()).then_some(paste.as_str());
            let pass_ref = if enc { Some(pass.as_str()) } else { None };
            match api::import::begin(&path, src, text, enc, pass_ref).await {
                Ok(rows) => {
                    // Default: import everything except rows that cannot be committed.
                    let init: HashMap<u32, bool> = rows
                        .iter()
                        .map(|r| (r.row_id, r.status != "error"))
                        .collect();
                    picked.set(init);
                    preview.set(rows);
                    report.set(None);
                }
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    let do_commit = move || {
        if busy.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        let rows = preview.get_untracked();
        let pk = picked.get_untracked();
        if rows.is_empty() || path.is_empty() {
            return;
        }
        let actions: Vec<ImportActionDto> = rows
            .iter()
            .map(|r| ImportActionDto {
                row_id: r.row_id,
                import: pk.get(&r.row_id).copied().unwrap_or(false),
            })
            .collect();
        let err_prefix = untrack(|| t_string!(i18n, settings.import_failed).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::import::commit(&path, &actions).await {
                Ok(rep) => {
                    report.set(Some(rep));
                    preview.set(Vec::new());
                    picked.set(HashMap::new());
                }
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    let do_cancel = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        preview.set(Vec::new());
        picked.set(HashMap::new());
        report.set(None);
        spawn_local(async move {
            let _ = api::import::cancel(&path).await;
        });
    };

    view! {
        // ---- Intro ----
        <div class="py-3.5 border-b border-border">
            <div class="text-sm font-medium text-text-primary">
                {move || t!(i18n, settings.import_title)}
            </div>
            <div class="text-xs text-text-secondary mt-0.5">
                {move || t!(i18n, settings.import_desc)}
            </div>
        </div>

        // ---- Source: encrypted toggle ----
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
                        {move || t!(i18n, settings.import_source_encrypted)}
                    </span>
                    <span class="block text-xs text-text-secondary">
                        {move || t!(i18n, settings.import_source_csv)}
                    </span>
                </span>
            </label>

            // Passphrase (encrypted source only).
            <Show when=move || encrypted.get() fallback=|| ()>
                <div class="pl-6">
                    <input
                        type="password"
                        class="w-full rounded-md border border-border bg-surface px-2.5 py-1.5 text-sm text-text-primary"
                        placeholder=move || t_string!(i18n, settings.import_passphrase_placeholder)
                        prop:value=move || passphrase.get()
                        on:input:target=move |ev| passphrase.set(ev.target().value())
                        data-testid="import-passphrase"
                    />
                </div>
            </Show>

            // File picker (path is pasteable, or Browse).
            <div class="pl-6 flex items-center gap-2">
                <input
                    type="text"
                    class="flex-1 rounded-md border border-border bg-surface px-2.5 py-1.5 text-sm text-text-primary"
                    placeholder=move || t_string!(i18n, settings.import_file_hint)
                    prop:value=move || src_path.get()
                    on:input:target=move |ev| src_path.set(ev.target().value())
                    data-testid="import-path"
                />
                <Button variant=Variant::Secondary on:click=move |_: web_sys::MouseEvent| browse()>
                    {move || t!(i18n, settings.import_browse)}
                </Button>
            </div>

            // ---- ⑧ Paste-into-VEdge — CSV only, RAM-only (no file). ----
            <Show when=move || !encrypted.get() fallback=|| ()>
                <div class="pl-6 space-y-1">
                    <div class="text-xs text-text-secondary">
                        {move || t!(i18n, settings.import_paste_label)}
                    </div>
                    <textarea
                        class="w-full h-24 rounded-md border border-border bg-surface px-2.5 py-1.5 text-xs font-mono text-text-primary"
                        placeholder=move || t_string!(i18n, settings.import_paste_placeholder)
                        prop:value=move || paste_text.get()
                        on:input:target=move |ev| paste_text.set(ev.target().value())
                        data-testid="import-paste"
                    ></textarea>
                </div>
            </Show>
        </div>

        // ---- Preview action ----
        <div class="flex items-center justify-end py-3.5 border-b border-border">
            {move || {
                let b = busy.get();
                view! {
                    <Button
                        variant=Variant::Primary
                        loading=b
                        disabled=b
                        attr:data-testid="import-preview"
                        on:click=move |_: web_sys::MouseEvent| do_preview()
                    >
                        {move || t!(i18n, settings.import_preview_button)}
                    </Button>
                }
            }}
        </div>

        // ---- Preview table ----
        <Show when=move || !preview.get().is_empty() fallback=|| ()>
            <div class="py-3.5 border-b border-border" data-testid="import-preview-table">
                <table class="w-full text-xs">
                    <tbody>
                        <For
                            each=move || preview.get()
                            key=|r| r.row_id
                            children=move |r: ImportPreviewRow| {
                                let rid = r.row_id;
                                let is_error = r.status == "error";
                                let ty = r.entry_type;
                                let name = r.name;
                                let username = r.username.unwrap_or_default();
                                let has_pw = r.has_password;
                                let dup = r.duplicate_of;
                                let status_badge = render_status(&r.status, r.status_message, dup);
                                let checked = move || {
                                    picked.get().get(&rid).copied().unwrap_or(false)
                                };
                                view! {
                                    <tr class="border-b border-border/50">
                                        <td class="py-1 pr-2 align-top">
                                            <input
                                                type="checkbox"
                                                prop:checked=checked
                                                disabled=is_error
                                                on:change:target=move |ev| {
                                                    let c = ev.target().checked();
                                                    picked
                                                        .update(|m| {
                                                            m.insert(rid, c);
                                                        });
                                                }
                                            />
                                        </td>
                                        <td class="py-1 pr-2 align-top text-text-secondary">
                                            {ty}
                                        </td>
                                        <td class="py-1 pr-2 align-top text-text-primary font-medium">
                                            {name}
                                            <span class="block text-text-secondary font-normal">
                                                {username}
                                            </span>
                                        </td>
                                        <td class="py-1 pr-2 align-top text-text-secondary">
                                            {if has_pw { "•••" } else { "" }}
                                        </td>
                                        <td class="py-1 align-top">{status_badge}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>

            // ---- Commit / Cancel ----
            <div class="flex items-center justify-end gap-2 py-3.5 border-b border-border">
                <Button
                    variant=Variant::Secondary
                    on:click=move |_: web_sys::MouseEvent| do_cancel()
                >
                    {move || t!(i18n, settings.import_cancel_button)}
                </Button>
                {move || {
                    let b = busy.get();
                    view! {
                        <Button
                            variant=Variant::Primary
                            loading=b
                            disabled=b
                            attr:data-testid="import-commit"
                            on:click=move |_: web_sys::MouseEvent| do_commit()
                        >
                            {move || t!(i18n, settings.import_commit_button)}
                        </Button>
                    }
                }}
            </div>
        </Show>

        // ---- Report ----
        {move || {
            report
                .get()
                .map(|r| {
                    let failed = r.failed.len();
                    view! {
                        <div class="py-3.5 text-xs text-text-secondary" data-testid="import-report">
                            <span class="font-medium text-text-primary">
                                {move || t!(i18n, settings.import_report_title)}
                            </span>
                            ": "
                            {r.imported.to_string()}
                            " "
                            {move || t!(i18n, settings.import_report_imported)}
                            " · "
                            {r.skipped.to_string()}
                            " "
                            {move || t!(i18n, settings.import_report_skipped)}
                            {(failed > 0)
                                .then(|| {
                                    view! {
                                        <span style="color:var(--color-danger-text)">
                                            " · " {failed.to_string()} " "
                                            {move || t!(i18n, settings.import_report_failed)}
                                        </span>
                                    }
                                })}
                        </div>
                    }
                })
        }}
    }
}

/// Render a row's status badge. `ok` → nothing; `warning`/`error` → the (English)
/// backend message; a `duplicate_of` adds an advisory hint. `+ use<>` so the
/// returned `impl IntoView` captures no borrow (Rust 2024 RPIT capture rule) — the
/// view owns its `text`.
fn render_status(
    status: &str,
    message: Option<String>,
    duplicate_of: Option<u32>,
) -> impl IntoView + use<> {
    let color = match status {
        "error" => "var(--color-danger-text)",
        "warning" => "var(--color-warning-text)",
        _ => "var(--color-text-secondary)",
    };
    let mut text = message.unwrap_or_default();
    if let Some(other) = duplicate_of {
        if !text.is_empty() {
            text.push_str(" · ");
        }
        text.push_str("duplicate of row ");
        text.push_str(&other.to_string());
    }
    view! { <span style=format!("color:{color}")>{text}</span> }
}
