use super::item::ToastRoot;
use super::types::{Toast, ToastInput, ToastState};
use std::time::Duration;
use uuid::Uuid;

use leptos::prelude::*;

impl ToastState {
    pub fn show(&self, input: ToastInput) -> Uuid {
        let id = Uuid::new_v4();
        let toast = Toast {
            id,
            title: input.title,
            description: input.description,
            duration_ms: input.duration_ms.unwrap_or(3000),
            color: input.color,
        };
        self.toasts.update(|v| v.push(toast.clone()));

        if toast.duration_ms > 0 {
            let toasts = self.toasts;
            leptos::prelude::set_timeout(
                move || toasts.update(|v| v.retain(|t| t.id != toast.id)),
                Duration::from_millis(toast.duration_ms),
            );
        }

        id
    }

    pub fn dismiss(&self, id: Uuid) {
        self.toasts.update(|v| v.retain(|t| t.id != id));
    }

    // pub fn clear(&self) {
    //     self.toasts.set(Vec::new());
    // }
}

pub fn use_toast() -> ToastState {
    expect_context::<ToastState>()
}
//
#[component]
pub fn ToastProvider(children: Children) -> impl IntoView {
    let state = ToastState {
        toasts: RwSignal::new(Vec::new()),
    };

    provide_context(state.clone());

    view! {
        {children()}
        <div
            class="fixed top-2 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-3"
            aria-live="polite"
            aria-atomic="true"
        >
            <For each=move || state.toasts.get()
                 key=|t| t.id
                 children=move |t| view! { <ToastRoot toast=t/> }/>
        </div>
    }
}
