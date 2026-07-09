use super::types::{ColorFormat, HsvColor, SwatchItem};
use leptos::prelude::*;

#[component]
pub(super) fn SwatchesGrid(
    swatches: Vec<SwatchItem>,
    hsv: RwSignal<HsvColor>,
    #[prop(into, default = None)] on_change_end: Option<Callback<String>>,
    format: ColorFormat,
    alpha: bool,
) -> impl IntoView {
    if swatches.is_empty() {
        return ().into_any();
    }

    // Pre-parse all swatches into HSV on mount (lazily, consumed once by the view)
    let parsed = swatches.into_iter().map(|sw| {
        let parsed = HsvColor::from_hex(&sw.value);
        (sw, parsed)
    });

    let fire_change_end = move || {
        if let Some(cb) = on_change_end {
            cb.run(hsv.get_untracked().format_output(format, alpha));
        }
    };

    view! {
        <div class="cp-divider"></div>
        <div class="cp-swatches" role="group" aria-label="Preset colors">
            {parsed
                .map(|(swatch, parsed_hsv)| {
                    let label = swatch.label.clone().unwrap_or_else(|| swatch.value.clone());
                    let hex_value = swatch.value;
                    let is_active = move || {
                        if let Some(ref p) = parsed_hsv {
                            let current = hsv.get();
                            current.to_hex(false) == p.to_hex(false)
                        } else {
                            false
                        }
                    };
                    let cls = move || {
                        if is_active() { "cp-swatch cp-swatch--active" } else { "cp-swatch" }
                    };
                    let on_click = {
                        let hex = hex_value.clone();
                        move |_: web_sys::MouseEvent| {
                            if let Some(parsed) = HsvColor::from_hex(&hex) {
                                hsv.set(
                                    if alpha { parsed } else { HsvColor { a: 100.0, ..parsed } },
                                );
                                fire_change_end();
                            }
                        }
                    };
                    view! {
                        <button
                            type="button"
                            class=cls
                            style=format!("background-color: {}", hex_value)
                            aria-label=label
                            aria-pressed=move || is_active().to_string()
                            on:click=on_click
                        />
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
