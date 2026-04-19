use super::item::ToastItem;
use super::types::{ToastData, ToastInput, ToastState};
use std::time::Duration;
use uuid::Uuid;

use leptos::prelude::*;

impl ToastState {
    pub fn show(&self, input: ToastInput) -> Uuid {
        #[cfg(debug_assertions)]
        if input.dismiss_label.is_empty() {
            web_sys::console::error_1(&"Toast: `dismiss_label` is required for i18n.".into());
        }

        let id = Uuid::new_v4();
        let toast = ToastData {
            id,
            variant: input.variant,
            message: input.message,
            action_label: input.action_label,
            on_action: input.on_action,
            duration: input.duration.unwrap_or(4000),
            on_dismiss: input.on_dismiss,
            dismiss_label: input.dismiss_label,
        };
        self.toasts.update(|v| v.push(toast));
        id
    }

    pub fn dismiss(&self, id: Uuid) {
        // Check if already dismissing
        let already = self.dismissing.with_untracked(|d| d.contains(&id));
        if already {
            return;
        }

        // Run on_dismiss callback
        if let Some(cb) = self.toasts.with_untracked(|toasts| {
            toasts
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.on_dismiss)
        }) {
            cb.run(());
        }

        // Trigger exit animation
        self.dismissing.update(|d| d.push(id));

        // Remove after exit animation (150ms)
        let toasts = self.toasts;
        let dismissing = self.dismissing;
        set_timeout(
            move || {
                toasts.update(|v| v.retain(|t| t.id != id));
                dismissing.update(|d| d.retain(|&did| did != id));
            },
            Duration::from_millis(150),
        );
    }
}

pub fn use_toast() -> ToastState {
    expect_context::<ToastState>()
}

#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    let state = ToastState {
        toasts: RwSignal::new(Vec::new()),
        dismissing: RwSignal::new(Vec::new()),
    };

    provide_context(state);

    view! {
        {children()}
        <div class="toast-group">
            <For
                each=move || state.toasts.get()
                key=|t| t.id
                children=move |t| view! { <ToastItem toast=t /> }
            />
        </div>
    }
}
