//! `ImportPreviewTable` — the shared derivatives-only preview grid (slice 5.3c).
//!
//! Extracted from the 5.3b import panel so the Settings ▸ Import tab and the
//! `/v/snapshots` tweezers render **identical** rows. Every source (encrypted
//! `.vedgex`, CSV, paste, or a snapshot) produces the same [`ImportPreviewRow`]
//! wire shape, so this component is source-agnostic: it renders a per-row
//! Import/Skip checkbox, the type + name/username, a `•••` placeholder when a
//! secret is present (Decision ⑦ — never the value), and a status badge.
//!
//! Pure data + reactive signals; no i18n context needed. The caller owns the
//! action buttons (Import/Recover · Cancel) and the report, so each surface keeps
//! its own labels and handlers.

use std::collections::HashMap;

use leptos::prelude::*;
use vedge_ipc::ImportPreviewRow;
use vedge_ui::components::Checkbox;

/// The preview grid, driven by the caller's `preview` + `picked` signals. Renders
/// nothing while `preview` is empty. `testid` sets the table's `data-testid` so each
/// surface can target it in e2e (`import-preview-table` in the panel, its own on the
/// snapshots page).
#[component]
pub fn ImportPreviewTable(
    preview: RwSignal<Vec<ImportPreviewRow>>,
    picked: RwSignal<HashMap<u32, bool>>,
    #[prop(optional, default = "import-preview-table")] testid: &'static str,
) -> impl IntoView {
    view! {
        <Show when=move || !preview.get().is_empty() fallback=|| ()>
            <div class="py-3.5 border-b border-border" data-testid=testid>
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
                                let name_aria = name.clone();
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
                                            <Checkbox
                                                checked=Signal::derive(checked)
                                                disabled=is_error
                                                on_change=Callback::new(move |c: bool| {
                                                    picked
                                                        .update(|m| {
                                                            m.insert(rid, c);
                                                        });
                                                })
                                                aria_label=name_aria
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
        </Show>
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
