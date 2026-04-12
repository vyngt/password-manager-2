use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Size, Status};
use leptos::prelude::*;
use web_sys::HtmlInputElement;

fn clamp(val: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let mut v = val;
    if let Some(lo) = min {
        if v < lo {
            v = lo;
        }
    }
    if let Some(hi) = max {
        if v > hi {
            v = hi;
        }
    }
    v
}

fn format_value(val: f64) -> String {
    if val == val.trunc() {
        format!("{}", val as i64)
    } else {
        format!("{}", val)
    }
}

#[component]
pub fn NumberInput(
    id: &'static str,
    #[prop(into, default = None)] value: Option<Signal<f64>>,
    #[prop(optional)] default_value: f64,
    #[prop(into, default = None)] min: Option<f64>,
    #[prop(into, default = None)] max: Option<f64>,
    #[prop(optional, default = 1.0)] step: f64,
    #[prop(optional, default = "")] unit: &'static str,
    #[prop(optional)] size: Size,
    #[prop(optional)] status: Status,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<f64>>,
    #[prop(optional, default = "")] aria_label: &'static str,
    #[prop(into, default = TextProp::default())] decrement_label: TextProp,
    #[prop(into, default = TextProp::default())] increment_label: TextProp,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if decrement_label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"NumberInput: `decrement_label` is required for i18n.".into(),
        );
    }

    #[cfg(debug_assertions)]
    if increment_label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"NumberInput: `increment_label` is required for i18n.".into(),
        );
    }

    let internal = RwSignal::new(default_value);
    let current_value = move || value.map(|s| s.get()).unwrap_or_else(|| internal.get());

    let input_ref = NodeRef::<leptos::html::Input>::new();

    let commit = move |new_val: f64| {
        let clamped = clamp(new_val, min, max);
        internal.set(clamped);
        if let Some(cb) = on_change {
            cb.run(clamped);
        }
    };

    let increment = move |multiplier: f64| {
        let new_val = current_value() + step * multiplier;
        commit(new_val);
    };

    let decrement = move |multiplier: f64| {
        let new_val = current_value() - step * multiplier;
        commit(new_val);
    };

    let can_decrement = move || min.map(|lo| current_value() > lo).unwrap_or(true);
    let can_increment = move || max.map(|hi| current_value() < hi).unwrap_or(true);

    let handle_blur = move |_: web_sys::FocusEvent| {
        if let Some(el) = input_ref.get() {
            let raw: &HtmlInputElement = &el;
            let text = raw.value();
            if let Ok(parsed) = text.parse::<f64>() {
                commit(parsed);
            }
            // Always sync display back to the canonical value
            raw.set_value(&format_value(current_value()));
        }
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                increment(1.0);
            }
            "ArrowDown" => {
                ev.prevent_default();
                decrement(1.0);
            }
            "PageUp" => {
                ev.prevent_default();
                increment(10.0);
            }
            "PageDown" => {
                ev.prevent_default();
                decrement(10.0);
            }
            "Home" => {
                if let Some(lo) = min {
                    ev.prevent_default();
                    commit(lo);
                }
            }
            "End" => {
                if let Some(hi) = max {
                    ev.prevent_default();
                    commit(hi);
                }
            }
            _ => {}
        }
    };

    let handle_decrement_click = move |_: web_sys::MouseEvent| {
        decrement(1.0);
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    let handle_increment_click = move |_: web_sys::MouseEvent| {
        increment(1.0);
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    };

    // CSS classes
    let root_cls = [
        "number-input",
        size.number_input_class(),
        status.number_input_class(),
        if disabled { "number-input--disabled" } else { "" },
        class,
    ]
    .join(" ");

    // ARIA
    let aria_label_attr = if aria_label.is_empty() { None } else { Some(aria_label) };
    let aria_describedby_attr = if aria_describedby.is_empty() {
        None
    } else {
        Some(aria_describedby)
    };

    view! {
        <div class=root_cls>
            // Decrement stepper
            <button
                type="button"
                class="number-input__stepper number-input__stepper--decrement"
                aria-label=move || decrement_label.get()
                disabled=move || disabled || !can_decrement()
                on:click=handle_decrement_click
                tabindex="-1"
            >
                <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
                    <line x1="2.5" y1="6" x2="9.5" y2="6" />
                </svg>
            </button>

            // Numeric input
            <input
                node_ref=input_ref
                class="number-input__field"
                id=id
                type="text"
                inputmode="numeric"
                role="spinbutton"
                disabled=disabled
                prop:value=move || format_value(current_value())
                aria-valuenow=move || current_value().to_string()
                aria-valuemin=min.map(|v| v.to_string())
                aria-valuemax=max.map(|v| v.to_string())
                aria-label=aria_label_attr
                aria-describedby=aria_describedby_attr
                on:blur=handle_blur
                on:keydown=handle_keydown
            />

            // Unit label
            {(!unit.is_empty()).then(|| view! {
                <span class="number-input__unit">{unit}</span>
            })}

            // Increment stepper
            <button
                type="button"
                class="number-input__stepper number-input__stepper--increment"
                aria-label=move || increment_label.get()
                disabled=move || disabled || !can_increment()
                on:click=handle_increment_click
                tabindex="-1"
            >
                <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
                    <line x1="2.5" y1="6" x2="9.5" y2="6" />
                    <line x1="6" y1="2.5" x2="6" y2="9.5" />
                </svg>
            </button>
        </div>
    }
}
