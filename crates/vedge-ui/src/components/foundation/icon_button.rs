use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Shape, Size, Variant};
use leptos::prelude::*;

#[component]
pub fn IconButton(
    children: Children,
    #[prop(into)] aria_label: TextProp,
    #[prop(optional, default = Variant::Ghost)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] shape: Shape,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional, default = "button")] button_type: &'static str,
    /// Extra classes. `TextProp` so consumers can pass a reactive `Signal<String>`
    /// (e.g. a selected/active state) as well as a static literal.
    #[prop(into, default = TextProp::default())]
    class: TextProp,
    /// Direct click handler, bound as `on:click` on the `<button>` itself.
    ///
    /// Prefer this over spreading `on:click` when the button is rendered inside a
    /// `<Portal>` (e.g. `Dialog`): Leptos's spread-onto-component forwarding does
    /// not reliably attach a listener across the portal boundary, so a spread
    /// `on:click` silently never fires there. This prop binds the handler directly
    /// on the element, which works everywhere. Event delegation is off by default
    /// in Leptos 0.8, so this coexists with any spread `on:click` (native listeners
    /// stack) — existing spread call sites are unaffected.
    #[prop(into, optional)]
    on_click: Option<Callback<()>>,
) -> impl IntoView {
    let is_disabled = disabled || loading;

    // Static base; the (possibly reactive) `class` prop is appended in the render
    // closure so a selected/active class can update without rebuilding the button.
    let base = [
        "icon-btn",
        variant.icon_btn_class(),
        size.icon_btn_class(),
        shape.icon_btn_class(),
        if loading { "icon-btn--loading" } else { "" },
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
            aria-label=move || aria_label.get()
            aria-busy=aria_busy_attr
            on:click=move |_: web_sys::MouseEvent| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {if loading {
                view! { <span class="icon-btn__spinner"></span> }.into_any()
            } else {
                children().into_any()
            }}
        </button>
    }
}
