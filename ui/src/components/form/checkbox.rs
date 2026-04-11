use crate::primitives::tokens::CheckboxSize;
use leptos::prelude::*;
use web_sys::HtmlInputElement;

#[component]
pub fn Checkbox(
    #[prop(into, default = None)] checked: Option<Signal<bool>>,
    #[prop(optional)] default_checked: bool,
    #[prop(into, default = None)] indeterminate: Option<Signal<bool>>,
    #[prop(optional)] size: CheckboxSize,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<bool>>,
    #[prop(optional, default = "")] aria_label: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let internal = RwSignal::new(default_checked);
    let is_checked = move || checked.map(|s| s.get()).unwrap_or_else(|| internal.get());
    let is_indeterminate = move || indeterminate.map(|s| s.get()).unwrap_or(false);

    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Sync indeterminate property to the native input (no HTML attribute exists)
    Effect::new(move || {
        let ind = is_indeterminate();
        if let Some(el) = input_ref.get() {
            let raw: &HtmlInputElement = &el;
            raw.set_indeterminate(ind);
        }
    });

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

    let root_cls = move || {
        [
            "checkbox",
            size.checkbox_class(),
            if is_indeterminate() {
                "checkbox--indeterminate"
            } else {
                ""
            },
            if disabled { "checkbox--disabled" } else { "" },
            class,
        ]
        .join(" ")
    };

    let aria_label_attr = if aria_label.is_empty() {
        None
    } else {
        Some(aria_label)
    };

    // Check icon SVG (polyline checkmark)
    let check_icon = move || {
        if is_indeterminate() {
            // Dash icon for indeterminate
            view! {
                <svg class="checkbox__icon" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <line x1="2.5" y1="6" x2="9.5" y2="6" />
                </svg>
            }
            .into_any()
        } else if is_checked() {
            // Checkmark icon
            view! {
                <svg class="checkbox__icon" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="2.5,6 5,8.5 9.5,3.5" />
                </svg>
            }
            .into_any()
        } else {
            view! {}.into_any()
        }
    };

    view! {
        <label class=root_cls>
            <input
                node_ref=input_ref
                class="checkbox__input"
                type="checkbox"
                disabled=disabled
                prop:checked=is_checked
                aria-checked=move || {
                    if is_indeterminate() { "mixed" } else if is_checked() { "true" } else { "false" }
                }
                aria-label=aria_label_attr
                on:change=handle_change
            />
            <span class="checkbox__box">
                {check_icon}
            </span>
        </label>
    }
}
