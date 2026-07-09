use super::types::{ColorFormat, HsvColor};
use leptos::prelude::*;
use web_sys::PointerEvent;

#[component]
pub(super) fn Gradient(
    hsv: RwSignal<HsvColor>,
    #[prop(into, default = None)] on_change_end: Option<Callback<String>>,
    format: ColorFormat,
    alpha: bool,
) -> impl IntoView {
    let gradient_ref = NodeRef::<leptos::html::Div>::new();
    let dragging = RwSignal::new(false);

    let update_from_pointer = move |ev: &PointerEvent| {
        if let Some(el) = gradient_ref.get() {
            let rect = el.get_bounding_client_rect();
            let x = ((f64::from(ev.client_x()) - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let y = ((f64::from(ev.client_y()) - rect.top()) / rect.height()).clamp(0.0, 1.0);
            hsv.update(|c| {
                c.s = x * 100.0;
                c.v = (1.0 - y) * 100.0;
            });
        }
    };

    let fire_change_end = move || {
        if let Some(cb) = on_change_end {
            cb.run(hsv.get_untracked().format_output(format, alpha));
        }
    };

    let on_pointerdown = move |ev: PointerEvent| {
        ev.prevent_default();
        if let Some(el) = gradient_ref.get() {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        dragging.set(true);
        update_from_pointer(&ev);
    };

    let on_pointermove = move |ev: PointerEvent| {
        if dragging.get() {
            update_from_pointer(&ev);
        }
    };

    let on_pointerup = move |ev: PointerEvent| {
        if dragging.get() {
            dragging.set(false);
            if let Some(el) = gradient_ref.get() {
                let _ = el.release_pointer_capture(ev.pointer_id());
            }
            fire_change_end();
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let step = if ev.shift_key() { 10.0 } else { 1.0 };
        let handled = match ev.key().as_str() {
            "ArrowLeft" => {
                hsv.update(|c| c.s = (c.s - step).clamp(0.0, 100.0));
                true
            }
            "ArrowRight" => {
                hsv.update(|c| c.s = (c.s + step).clamp(0.0, 100.0));
                true
            }
            "ArrowUp" => {
                hsv.update(|c| c.v = (c.v + step).clamp(0.0, 100.0));
                true
            }
            "ArrowDown" => {
                hsv.update(|c| c.v = (c.v - step).clamp(0.0, 100.0));
                true
            }
            _ => false,
        };
        if handled {
            ev.prevent_default();
            fire_change_end();
        }
    };

    // Derived CSS values
    let hue_style = move || format!("--cp-hue: {}", hsv.get().h.round() as i32);
    let thumb_style = move || {
        let c = hsv.get();
        format!("left: {}%; top: {}%", c.s, 100.0 - c.v)
    };
    let aria_valuetext = move || {
        let c = hsv.get();
        format!(
            "Saturation {}%, Brightness {}%",
            c.s.round() as i32,
            c.v.round() as i32
        )
    };

    view! {
        <div
            node_ref=gradient_ref
            class="cp-gradient"
            style=hue_style
            tabindex=0
            role="slider"
            aria-label="Saturation and brightness"
            aria-valuetext=aria_valuetext
            on:pointerdown=on_pointerdown
            on:pointermove=on_pointermove
            on:pointerup=on_pointerup
            on:keydown=on_keydown
        >
            <div class="cp-gradient__thumb" style=thumb_style />
        </div>
    }
}
