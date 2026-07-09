use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Orientation;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// A single radio option.
#[derive(Clone, Debug)]
pub struct RadioOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl RadioOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

#[component]
pub fn RadioGroup(
    options: Vec<RadioOption>,
    #[prop(into, default = None)] value: Option<Signal<String>>,
    #[prop(optional, default = "")] default_value: &'static str,
    #[prop(optional)] orientation: Orientation,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    #[prop(optional, default = "")] name: &'static str,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    // Generate a unique name if none provided, so multiple groups don't collide.
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let group_name: String = if name.is_empty() {
        format!("radio-group-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    } else {
        name.to_owned()
    };

    let internal = RwSignal::new(default_value.to_owned());
    let selected = move || value.map_or_else(|| internal.get(), |s| s.get());

    let option_count = options.len();
    let options = StoredValue::new(options);

    let select_value = move |val: String| {
        internal.set(val.clone());
        if let Some(cb) = on_change {
            cb.run(val);
        }
    };

    // Keyboard handler for roving tabindex.
    #[allow(
        clippy::indexing_slicing,
        reason = "indices are (idx ± 1) % option_count == opts.len() — provably in bounds; \
                  rewriting to `.get()` would force a dead `None` branch on selection"
    )]
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        let is_nav = matches!(
            key.as_str(),
            "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
        );
        if !is_nav || option_count == 0 {
            return;
        }
        ev.prevent_default();

        let opts = options.get_value();
        let current_val = selected();
        let current_idx = opts
            .iter()
            .position(|o| o.value == current_val)
            .unwrap_or(0);

        let next_idx = match key.as_str() {
            "ArrowDown" | "ArrowRight" => {
                // Find next non-disabled option, wrapping
                let mut idx = current_idx;
                for _ in 0..option_count {
                    idx = (idx + 1) % option_count;
                    if !opts[idx].disabled || disabled {
                        break;
                    }
                }
                idx
            }
            "ArrowUp" | "ArrowLeft" => {
                let mut idx = current_idx;
                for _ in 0..option_count {
                    idx = if idx == 0 { option_count - 1 } else { idx - 1 };
                    if !opts[idx].disabled || disabled {
                        break;
                    }
                }
                idx
            }
            _ => return,
        };

        let new_val = opts[next_idx].value.clone();
        select_value(new_val);

        // Focus the newly selected input
        if let Some(target) = ev.current_target() {
            if let Some(group_el) = target.dyn_ref::<web_sys::HtmlElement>() {
                let selector = format!("input[value=\"{}\"]", opts[next_idx].value);
                if let Ok(Some(input)) = group_el.query_selector(&selector) {
                    if let Some(el) = input.dyn_ref::<web_sys::HtmlElement>() {
                        let _ = el.focus();
                    }
                }
            }
        }
    };

    let root_cls = [
        "radio-group",
        orientation.radio_group_class(),
        if disabled {
            "radio-group--disabled"
        } else {
            ""
        },
        class,
    ]
    .join(" ");

    view! {
        <div
            class=root_cls
            role="radiogroup"
            aria-label=move || {
                let v = aria_label.get();
                if v.is_empty() { None } else { Some(v) }
            }
            on:keydown=handle_keydown
        >
            {options
                .get_value()
                .into_iter()
                .enumerate()
                .map(|(i, opt)| {
                    let val = opt.value.clone();
                    let val2 = opt.value.clone();
                    let val3 = opt.value.clone();
                    let label = opt.label.clone();
                    let input_name = group_name.clone();
                    let opt_disabled = opt.disabled;
                    let item_disabled = disabled || opt_disabled;
                    let is_selected = {
                        let v = val;
                        move || selected() == v
                    };
                    let item_cls = move || {
                        ["radio-item", if item_disabled { "radio-item--disabled" } else { "" }]
                            .join(" ")
                    };
                    let is_selected2 = is_selected.clone();
                    let tab_idx = move || {
                        if is_selected2() || (selected().is_empty() && i == 0) { 0 } else { -1 }
                    };
                    let on_change_handler = move |_: web_sys::Event| {
                        if !item_disabled {
                            select_value(val2.clone());
                        }
                    };

                    // Roving tabindex: only selected (or first if none selected) gets 0

                    view! {
                        <label class=item_cls>
                            <input
                                class="radio-item__input"
                                type="radio"
                                name=input_name
                                value=val3
                                disabled=item_disabled
                                prop:checked=is_selected
                                tabindex=tab_idx
                                on:change=on_change_handler
                            />
                            <span class="radio-item__circle"></span>
                            <span class="radio-item__label">{label}</span>
                        </label>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}
