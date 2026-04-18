use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::CheckboxSize;
use leptos::prelude::*;
use web_sys::HtmlInputElement;

#[component]
pub fn Toggle(
    #[prop(into, default = None)] checked: Option<Signal<bool>>,
    #[prop(optional)] default_checked: bool,
    #[prop(optional)] size: CheckboxSize,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<bool>>,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] aria_labelledby: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let internal = RwSignal::new(default_checked);
    let is_checked = move || checked.map(|s| s.get()).unwrap_or_else(|| internal.get());

    let input_ref = NodeRef::<leptos::html::Input>::new();

    let handle_change = move |_: web_sys::Event| {
        if let Some(el) = input_ref.get() {
            let raw: &HtmlInputElement = &el;
            let new_val = raw.checked();
            internal.set(new_val);
            if let Some(cb) = on_change {
                cb.run(new_val);
            }
        }
    };

    let root_cls = [
        "toggle",
        size.toggle_class(),
        if disabled { "toggle--disabled" } else { "" },
        class,
    ]
    .join(" ");

    let aria_labelledby_attr = if aria_labelledby.is_empty() {
        None
    } else {
        Some(aria_labelledby)
    };

    view! {
        <label class=root_cls role="switch" aria-checked=move || is_checked().to_string()>
            <input
                node_ref=input_ref
                class="toggle__input"
                type="checkbox"
                disabled=disabled
                prop:checked=is_checked
                aria-label=move || {
                    let v = aria_label.get();
                    if v.is_empty() { None } else { Some(v) }
                }
                aria-labelledby=aria_labelledby_attr
                on:change=handle_change
            />
            <span class="toggle__track">
                <span class="toggle__thumb"></span>
            </span>
        </label>
    }
}
