//! One-time Secret Key display: monospace block + copy button.
//!
//! Renders the Emergency-Kit `A3-XXXXX-…` string (and, reused, a revealed
//! entry-history secret). `vedge-ui` has no secret-display component, so this
//! composes a mono `<code>` block with the existing `CopyButton`.
//!
//! **Copy** routes through the hardened native `copy_text` command (slice 3.2)
//! via `CopyButton`'s `copy_with` strategy, so the copied secret carries the OS
//! no-history / no-cloud exclusion hints and is auto-cleared after the
//! configured delay — the same discipline as the vault's field copies. The
//! button's icon swaps optimistically; a toast reports the real outcome.

use leptos::prelude::*;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{CopyButton, CopyFuture};
use vedge_ui::primitives::tokens::ToastVariant;

use crate::api;
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::i18n::{t_string, use_i18n};

#[component]
pub fn SecretDisplay(#[prop(into)] value: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let sec = expect_context::<SecurityPrefsCtx>();
    let toast = use_toast();

    let copy_native = Callback::new(move |v: String| -> CopyFuture {
        // Read prefs + locale strings in the handler body (reactive owner
        // present); reading them inside the future would trip the "outside a
        // reactive tracking context" warning.
        let secs = sec.0.get_untracked().clipboard_clear_seconds;
        let ok_msg = t_string!(i18n, vault.copied).to_owned();
        let err_msg = t_string!(i18n, vault.copy_failed).to_owned();
        let dismiss = t_string!(i18n, vault.dismiss).to_owned();
        Box::pin(async move {
            let (msg, variant, ok) = match api::clipboard::copy_text(&v, Some(secs)).await {
                Ok(()) => (ok_msg, ToastVariant::Success, true),
                Err(_) => (err_msg, ToastVariant::Danger, false),
            };
            toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
            ok
        })
    });

    view! {
        <div class="flex items-center gap-2 rounded-md border border-border bg-primary-muted p-3">
            // `whitespace-pre-wrap` preserves newlines: a revealed multi-line PEM
            // (SSH private key, slice 5.4.1) must render across lines, not collapse
            // to one via the default `white-space: normal`. Harmless for the
            // single-line Secret-Key string this also renders.
            <code class="flex-1 select-all whitespace-pre-wrap break-all font-mono text-sm text-text-primary">
                {move || value.get()}
            </code>
            <CopyButton value=value copy_with=copy_native />
        </div>
    }
}
