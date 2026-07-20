use super::types::{HsvColor, hsv_to_hsl};
use leptos::prelude::*;

#[component]
pub(super) fn HueSlider(hsv: RwSignal<HsvColor>) -> impl IntoView {
    let hue_value = move || hsv.get().h.round().to_string();

    let on_input = move |ev: leptos::ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Ok(val) = target.value().parse::<f64>() {
            hsv.update(|c| c.h = val.clamp(0.0, 360.0));
        }
    };

    let aria_valuetext = move || format!("{} degrees", hsv.get().h.round() as i32);

    view! {
        <input
            type="range"
            class="cp-slider cp-hue-slider"
            min="0"
            max="360"
            step="1"
            prop:value=hue_value
            on:input=on_input
            aria-label="Hue"
            aria-valuetext=aria_valuetext
        />
    }
}

#[component]
pub(super) fn AlphaSlider(hsv: RwSignal<HsvColor>) -> impl IntoView {
    let alpha_value = move || hsv.get().a.round().to_string();

    let on_input = move |ev: leptos::ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&ev);
        if let Ok(val) = target.value().parse::<f64>() {
            hsv.update(|c| c.a = val.clamp(0.0, 100.0));
        }
    };

    let aria_valuetext = move || format!("{}%", hsv.get().a.round() as i32);

    // Track background needs the solid color (without alpha) as a CSS variable
    let track_style = move || {
        let c = hsv.get();
        let (h, s, l) = hsv_to_hsl(c.h, c.s, c.v);
        format!(
            "--cp-solid-color: hsl({}, {}%, {}%)",
            h.round() as i32,
            s.round() as i32,
            l.round() as i32
        )
    };

    view! {
        <input
            type="range"
            class="cp-slider cp-alpha-slider"
            style=track_style
            min="0"
            max="100"
            step="1"
            prop:value=alpha_value
            on:input=on_input
            aria-label="Opacity"
            aria-valuetext=aria_valuetext
        />
    }
}
