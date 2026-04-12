use leptos::prelude::*;

#[component]
pub fn Section(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
            <h3 class="text-xs font-medium text-text-tertiary uppercase tracking-wide">{title}</h3>
            {children()}
        </div>
    }
}
