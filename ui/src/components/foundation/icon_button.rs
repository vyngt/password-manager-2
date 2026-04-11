use crate::primitives::tokens::{Shape, Size, Variant};
use leptos::prelude::*;

#[component]
pub fn IconButton(
    children: Children,
    #[prop(into)] aria_label: String,
    #[prop(optional, default = Variant::Ghost)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] shape: Shape,
    #[prop(optional, default = "")] tooltip: &'static str,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let is_disabled = disabled || loading;

    let cls = [
        "icon-btn",
        variant.icon_btn_class(),
        size.icon_btn_class(),
        shape.icon_btn_class(),
        if loading { "icon-btn--loading" } else { "" },
        class,
    ]
    .join(" ");

    let title_attr = if !tooltip.is_empty() {
        tooltip.to_string()
    } else {
        aria_label.clone()
    };

    let aria_busy_attr = if loading { Some("true") } else { None };

    view! {
        <button
            type=button_type
            class=cls
            disabled=is_disabled
            aria-label=aria_label
            aria-busy=aria_busy_attr
            title=title_attr
        >
            {if loading {
                view! { <span class="icon-btn__spinner"></span> }.into_any()
            } else {
                children().into_any()
            }}
        </button>
    }
}
