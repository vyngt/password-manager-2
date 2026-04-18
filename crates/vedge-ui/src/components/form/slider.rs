use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Size;
use leptos::prelude::*;
use web_sys::{KeyboardEvent, PointerEvent};

#[component]
pub fn Slider(
    #[prop(optional, default = 0.0)] min: f64,
    #[prop(optional, default = 100.0)] max: f64,
    #[prop(optional, default = 1.0)] step: f64,

    #[prop(into, default = None)] value: Option<Signal<f64>>,
    #[prop(optional, default = 0.0)] default_value: f64,

    #[prop(optional)] size: Size,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] show_value_label: bool,

    #[prop(into, default = None)] format_label: Option<Callback<f64, String>>,

    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] aria_labelledby: &'static str,

    #[prop(into, default = None)] on_change: Option<Callback<f64>>,
    #[prop(into, default = None)] on_change_end: Option<Callback<f64>>,

    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    // --- Dev assertions ---
    #[cfg(debug_assertions)]
    {
        if step <= 0.0 {
            web_sys::console::warn_1(&"Slider: `step` must be positive.".into());
        }
        if !(min < max) {
            web_sys::console::warn_1(&"Slider: `min` must be less than `max`.".into());
        }
        if show_value_label && format_label.is_none() {
            web_sys::console::warn_1(
                &"Slider: `format_label` is required when `show_value_label` is true.".into(),
            );
        }
    }

    // --- State ---
    let step = if step > 0.0 { step } else { 1.0 };
    let internal = RwSignal::new(default_value.clamp(min, max));
    let current = Memo::new(move |_| {
        value
            .map(|s| s.get())
            .unwrap_or_else(|| internal.get())
            .clamp(min, max)
    });

    let dragging = RwSignal::new(false);
    let track_ref = NodeRef::<leptos::html::Div>::new();

    // --- Helpers ---
    let snap = move |raw: f64| -> f64 {
        let n = ((raw - min) / step).round();
        let v = (min + n * step).clamp(min, max);
        // Round to 10 decimals to scrub float drift for decimal steps.
        (v * 1e10).round() / 1e10
    };

    let commit = move |raw: f64, is_end: bool| {
        let v = snap(raw);
        if value.is_none() {
            internal.set(v);
        }
        if let Some(cb) = on_change {
            cb.run(v);
        }
        if is_end {
            if let Some(cb) = on_change_end {
                cb.run(v);
            }
        }
    };

    let value_from_pointer_x = move |client_x: f64| -> f64 {
        if let Some(el) = track_ref.get() {
            let rect = el.get_bounding_client_rect();
            let width = rect.width().max(1.0);
            let frac = ((client_x - rect.left()) / width).clamp(0.0, 1.0);
            min + frac * (max - min)
        } else {
            current.get_untracked()
        }
    };

    // --- Event handlers ---
    let on_pointerdown = move |ev: PointerEvent| {
        if disabled {
            return;
        }
        ev.prevent_default();
        if let Some(el) = track_ref.get() {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(true);
        let raw = value_from_pointer_x(ev.client_x() as f64);
        commit(raw, false);
    };

    let on_pointermove = move |ev: PointerEvent| {
        if !dragging.get_untracked() {
            return;
        }
        let raw = value_from_pointer_x(ev.client_x() as f64);
        commit(raw, false);
    };

    let on_pointerup = move |ev: PointerEvent| {
        if !dragging.get_untracked() {
            return;
        }
        dragging.set(false);
        if let Some(el) = track_ref.get() {
            let _ = el.release_pointer_capture(ev.pointer_id());
        }
        let raw = value_from_pointer_x(ev.client_x() as f64);
        commit(raw, true);
    };

    let on_pointercancel = move |_ev: PointerEvent| {
        if dragging.get_untracked() {
            dragging.set(false);
            // Fire end with the last known value (no new position).
            let v = current.get_untracked();
            if let Some(cb) = on_change_end {
                cb.run(v);
            }
        }
    };

    let on_keydown = move |ev: KeyboardEvent| {
        if disabled {
            return;
        }
        let v = current.get_untracked();
        let key = ev.key();
        match key.as_str() {
            "ArrowRight" | "ArrowUp" => {
                ev.prevent_default();
                commit(v + step, true);
            }
            "ArrowLeft" | "ArrowDown" => {
                ev.prevent_default();
                commit(v - step, true);
            }
            "PageUp" => {
                ev.prevent_default();
                commit(v + step * 10.0, true);
            }
            "PageDown" => {
                ev.prevent_default();
                commit(v - step * 10.0, true);
            }
            "Home" => {
                ev.prevent_default();
                commit(min, true);
            }
            "End" => {
                ev.prevent_default();
                commit(max, true);
            }
            _ => {}
        }
    };

    // --- Derived view fragments ---
    let root_cls = [
        "slider-root",
        size.slider_root_class(),
        if show_value_label { "slider-root--with-label" } else { "" },
        class,
    ]
    .join(" ");

    let fill_style = move || {
        let pct = ((current.get() - min) / (max - min) * 100.0).clamp(0.0, 100.0);
        format!("width: {pct}%")
    };

    let thumb_style = move || {
        let pct = ((current.get() - min) / (max - min) * 100.0).clamp(0.0, 100.0);
        format!("left: {pct}%")
    };

    let dragging_attr = move || dragging.get().then_some("true");
    let disabled_attr = move || disabled.then_some("true");
    let tab_index = if disabled { "-1" } else { "0" };

    let aria_label_attr = move || {
        let v = aria_label.get();
        if v.is_empty() { None } else { Some(v) }
    };
    let aria_labelledby_attr =
        if aria_labelledby.is_empty() { None } else { Some(aria_labelledby) };

    let aria_valuenow = move || format!("{}", current.get());
    let aria_valuetext = move || format_label.map(|f| f.run(current.get()));
    let aria_valuemin = format!("{min}");
    let aria_valuemax = format!("{max}");

    let value_label_node = move || {
        if show_value_label {
            format_label.map(|f| {
                let label = move || f.run(current.get());
                view! {
                    <span class="slider-value-label" aria-hidden="true">
                        {label}
                    </span>
                }
            })
        } else {
            None
        }
    };

    view! {
        <div class=root_cls data-disabled=disabled_attr>
            <div
                class="slider-track"
                node_ref=track_ref
                on:pointerdown=on_pointerdown
                on:pointermove=on_pointermove
                on:pointerup=on_pointerup
                on:pointercancel=on_pointercancel
            >
                <div class="slider-fill" style=fill_style></div>
                <div
                    class="slider-thumb"
                    role="slider"
                    tabindex=tab_index
                    aria-valuemin=aria_valuemin
                    aria-valuemax=aria_valuemax
                    aria-valuenow=aria_valuenow
                    aria-valuetext=aria_valuetext
                    aria-label=aria_label_attr
                    aria-labelledby=aria_labelledby_attr
                    aria-disabled=disabled_attr
                    data-dragging=dragging_attr
                    style=thumb_style
                    on:keydown=on_keydown
                >
                    {value_label_node}
                </div>
            </div>
        </div>
    }
}
