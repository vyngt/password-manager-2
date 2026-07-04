//! One-time Secret Key display: monospace block + copy button.
//!
//! Renders the Emergency-Kit `A3-XXXXX-…` string. `vedge-ui` has no
//! secret-display component, so this composes a mono `<code>` block with the
//! existing `CopyButton`.

use leptos::prelude::*;
use vedge_ui::components::CopyButton;

#[component]
pub fn SecretDisplay(#[prop(into)] value: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 rounded-md border border-border bg-primary-muted p-3">
            <code class="flex-1 select-all break-all font-mono text-sm text-text-primary">
                {move || value.get()}
            </code>
            <CopyButton value=value />
        </div>
    }
}
