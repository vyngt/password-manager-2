use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Shape, Size, Variant};
use leptos::prelude::*;

#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] shape: Shape,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    #[prop(optional)] full_width: bool,
    /// Extra classes. `TextProp` so consumers can pass a reactive `Signal<String>`
    /// (e.g. a selected/active state) as well as a static literal.
    #[prop(into, default = TextProp::default())]
    class: TextProp,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
) -> impl IntoView {
    let is_disabled = disabled || loading;

    // Static base; the (possibly reactive) `class` prop is appended in the render
    // closure so a selected/active class can update without rebuilding the button.
    let base = [
        "btn",
        variant.btn_class(),
        size.btn_class(),
        shape.btn_class(),
        if full_width { "btn--full" } else { "" },
        if loading { "btn--loading" } else { "" },
    ]
    .join(" ");

    let aria_busy_attr = if loading { Some("true") } else { None };

    view! {
        <button
            type=button_type
            class=move || {
                let extra = class.get();
                if extra.is_empty() { base.clone() } else { format!("{base} {extra}") }
            }
            disabled=is_disabled
            aria-busy=aria_busy_attr
            aria-label=move || {
                let v = aria_label.get();
                if v.is_empty() { None } else { Some(v) }
            }
        >
            {loading.then(|| view! { <span class="btn__spinner"></span> })}
            {children()}
        </button>
    }
}
