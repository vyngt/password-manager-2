use leptos::prelude::*;
use web_sys::wasm_bindgen::JsCast;

use crate::types::color::RgbColor;

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

    // Remove old ripple if exists
    if let Some(old) = target.query_selector(".ripple").unwrap() {
        target.remove_child(&old).ok();
    }

    target.append_child(&ripple).unwrap();
}
