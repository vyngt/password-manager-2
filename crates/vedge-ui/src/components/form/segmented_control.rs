use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Size;
use leptos::prelude::*;
use leptos_icons::Icon;
use wasm_bindgen::JsCast;

/// A single segment option.
#[derive(Clone, Debug)]
pub struct SegmentOption {
    pub value: String,
    pub label: Option<String>,
    pub icon: Option<icondata_core::Icon>,
    pub aria_label: Option<String>,
    pub disabled: bool,
}

impl SegmentOption {
    pub fn text(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: Some(label.into()),
            icon: None,
            aria_label: None,
            disabled: false,
        }
    }

    pub fn icon(value: impl Into<String>, icon: icondata_core::Icon, aria_label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: None,
            icon: Some(icon),
            aria_label: Some(aria_label.into()),
            disabled: false,
        }
    }

    pub fn icon_text(
        value: impl Into<String>,
        icon: icondata_core::Icon,
        label: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: Some(label.into()),
            icon: Some(icon),
            aria_label: None,
            disabled: false,
        }
    }
}

#[component]
pub fn SegmentedControl(
    options: Vec<SegmentOption>,
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] size: Size,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if size == Size::Lg {
        web_sys::console::warn_1(
            &"SegmentedControl: Size::Lg is not supported. Using Md.".into(),
        );
    }

    let option_count = options.len();
    let options = StoredValue::new(options);

    // Indicator position state
    let indicator_style = RwSignal::new(String::new());
    let root_ref = NodeRef::<leptos::html::Div>::new();

    // Update indicator position when value changes.
    // Uses request_animation_frame so the DOM has laid out before we read offsets.
    Effect::new(move || {
        let val = value.get(); // reactive tracking happens here
        request_animation_frame(move || {
            if let Some(root) = root_ref.get_untracked() {
                let raw: &web_sys::HtmlElement = &root;
                let selector = format!("[data-value=\"{}\"]", val);
                if let Ok(Some(el)) = raw.query_selector(&selector) {
                    if let Some(btn) = el.dyn_ref::<web_sys::HtmlElement>() {
                        let left = btn.offset_left();
                        let width = btn.offset_width();
                        indicator_style.set(format!(
                            "transform: translateX({}px); width: {}px;",
                            left, width
                        ));
                    }
                }
            }
        });
    });

    let select_value = move |val: String| {
        if let Some(cb) = on_change {
            cb.run(val);
        }
    };

    // Keyboard handler (roving tabindex)
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        let is_nav = matches!(key.as_str(), "ArrowLeft" | "ArrowRight");
        if !is_nav || option_count == 0 {
            return;
        }
        ev.prevent_default();

        let opts = options.get_value();
        let current_val = value.get_untracked();
        let current_idx = opts
            .iter()
            .position(|o| o.value == current_val)
            .unwrap_or(0);

        let next_idx = match key.as_str() {
            "ArrowRight" => {
                let mut idx = current_idx;
                for _ in 0..option_count {
                    idx = (idx + 1) % option_count;
                    if !opts[idx].disabled {
                        break;
                    }
                }
                idx
            }
            "ArrowLeft" => {
                let mut idx = current_idx;
                for _ in 0..option_count {
                    idx = if idx == 0 { option_count - 1 } else { idx - 1 };
                    if !opts[idx].disabled {
                        break;
                    }
                }
                idx
            }
            _ => return,
        };

        let new_val = opts[next_idx].value.clone();
        select_value(new_val);

        // Focus the newly selected button
        if let Some(root) = root_ref.get() {
            let raw: &web_sys::HtmlElement = &root;
            let selector = format!("[data-value=\"{}\"]", opts[next_idx].value);
            if let Ok(Some(el)) = raw.query_selector(&selector) {
                if let Some(btn) = el.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = btn.focus();
                }
            }
        }
    };

    let root_cls = [
        "segmented",
        size.segmented_class(),
        if disabled { "segmented--disabled" } else { "" },
        class,
    ]
    .join(" ");

    view! {
        <div
            node_ref=root_ref
            class=root_cls
            role="radiogroup"
            aria-label=move || { let v = aria_label.get(); if v.is_empty() { None } else { Some(v) } }
            on:keydown=handle_keydown
        >
            <span
                class="segmented__indicator"
                style=move || indicator_style.get()
                aria-hidden="true"
            ></span>
            {options
                .get_value()
                .into_iter()
                .map(|opt| {
                    let val = opt.value.clone();
                    let val2 = opt.value.clone();
                    let opt_disabled = opt.disabled;
                    let item_disabled = disabled || opt_disabled;

                    let is_active = {
                        let v = val.clone();
                        move || value.get() == v
                    };

                    let is_active2 = is_active.clone();
                    let tab_idx = move || if is_active2() { 0 } else { -1 };

                    let on_click = {
                        let select_value = select_value.clone();
                        let val = val2.clone();
                        move |_: web_sys::MouseEvent| {
                            if !item_disabled {
                                select_value(val.clone());
                            }
                        }
                    };

                    view! {
                        <button
                            class="segmented__btn"
                            role="radio"
                            type="button"
                            aria-checked=move || is_active().to_string()
                            aria-label=opt.aria_label.clone()
                            data-value=val2
                            disabled=item_disabled
                            tabindex=tab_idx
                            on:click=on_click
                        >
                            {opt.icon.map(|icon| view! { <Icon icon=icon /> })}
                            {opt.label.clone().map(|label| view! { <span>{label}</span> })}
                        </button>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}
