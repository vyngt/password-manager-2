use crate::primitives::tokens::{Size, Variant};
use leptos::prelude::*;

#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional)] full_width: bool,
    #[prop(optional, default = "")] class: &'static str,
    #[prop(optional, default = "")] aria_label: &'static str,
) -> impl IntoView {
    let is_disabled = disabled || loading;

    let cls = [
        "btn",
        variant.btn_class(),
        size.btn_class(),
        if full_width { "btn-full-width" } else { "" },
        if loading { "btn-loading" } else { "" },
        class,
    ]
    .join(" ");

    let aria_label_attr = if aria_label.is_empty() {
        None
    } else {
        Some(aria_label)
    };
    let aria_busy_attr = if loading { Some("true") } else { None };

    view! {
        <button
            type=button_type
            class=cls
            disabled=is_disabled
            aria-busy=aria_busy_attr
            aria-label=aria_label_attr
        >
            {loading.then(|| view! { <span class="btn-spinner"></span> })}
            {children()}
        </button>
    }
}
