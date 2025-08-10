use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::JsCast;

use crate::types::color::RgbColor;

static RIPPLE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq)]
pub struct RippleColor {
    pub color: RgbColor,
    pub alpha: f32,
}

pub fn add_ripple(ev: web_sys::MouseEvent, color: Option<RippleColor>) {
    let ripple_color = color.unwrap_or(RippleColor {
        color: RgbColor::new(255, 255, 255),
        alpha: 0.6,
    });

    let target = event_target::<web_sys::HtmlElement>(&ev);
    let ripple = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("span")
        .unwrap();

    // Generate unique ID for this ripple
    let ripple_id = format!("ripple-{}", RIPPLE_COUNTER.fetch_add(1, Ordering::Relaxed));
    ripple.set_id(&ripple_id);

    ripple
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property(
            "--ripple-color",
            &format!(
                "rgba({}, {}, {}, {})",
                ripple_color.color.r,
                ripple_color.color.g,
                ripple_color.color.b,
                ripple_color.alpha
            ),
        )
        .unwrap();

    let rect = target.get_bounding_client_rect();

    let diameter = rect.width().max(rect.height());
    ripple
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property("width", &format!("{diameter}px"))
        .unwrap();
    ripple
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property("height", &format!("{diameter}px"))
        .unwrap();

    let x = ev.client_x() as f64 - rect.left() - diameter / 2.0;
    let y = ev.client_y() as f64 - rect.top() - diameter / 2.0;

    ripple.set_class_name("ripple");
    ripple
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property("left", &format!("{x}px"))
        .unwrap();
    ripple
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property("top", &format!("{y}px"))
        .unwrap();

    // Clean up old completed ripples (older than 1 second)
    cleanup_old_ripples(&target);

    target.append_child(&ripple).unwrap();

    // Set up automatic cleanup using animation end event
    let target_clone = target.clone();
    let ripple_id_clone = ripple_id.clone();

    // Add animation end event listener for automatic cleanup
    let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if let Some(ripple_element) = target_clone
            .query_selector(&format!("#{}", ripple_id_clone))
            .unwrap()
        {
            target_clone.remove_child(&ripple_element).ok();
        }
    }) as Box<dyn FnMut(web_sys::Event)>);

    ripple
        .add_event_listener_with_callback("animationend", closure.as_ref().unchecked_ref())
        .unwrap();

    // Keep the closure alive
    closure.forget();
}

fn cleanup_old_ripples(target: &web_sys::HtmlElement) {
    // Remove any ripples that don't have an ID (legacy ripples)
    if let Some(old_ripple) = target.query_selector(".ripple:not([id])").unwrap() {
        target.remove_child(&old_ripple).ok();
    }
}
