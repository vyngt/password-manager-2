use crate::components::foundation::icon_button::IconButton;

use super::types::{Toast, ToastState};

use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn ToastRoot(toast: Toast) -> impl IntoView {
    let cls: String = vec!["toast"].join("");

    let handle_style = move || {
        let c = toast.color;
        let color_var = format!("--toast-color: rgb({}, {}, {})", c.r, c.g, c.b);
        format!("{};", color_var)
    };

    view! {
        <div class=cls style=handle_style>
            <div class="toast-inner">
                <div class="flex-1">
                    <ToastTitle>{toast.title.clone()}</ToastTitle>
                    {toast
                        .description
                        .map(|d| view! { <ToastDescription>{d.clone()}</ToastDescription> })}
                </div>
                <ToastClose id=toast.id />
            </div>
        </div>
    }
}

#[component]
pub fn ToastTitle(children: Children) -> impl IntoView {
    view! { <div class="font-semibold text-base">{children()}</div> }
}

#[component]
pub fn ToastDescription(children: Children) -> impl IntoView {
    view! { <div class="text-sm opacity-90">{children()}</div> }
}

#[component]
pub fn ToastClose(id: uuid::Uuid) -> impl IntoView {
    let state = expect_context::<ToastState>();
    view! {
        <IconButton
            aria_label="Close"
            class="text-[var(--toast-color)]"
            on:click=move |_| state.dismiss(id)
        >
            <Icon icon=i::FaXmarkSolid />
        </IconButton>
    }
}
