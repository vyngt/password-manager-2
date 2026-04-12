use super::format_row::FormatRow;
use super::gradient::Gradient;
use super::sliders::{AlphaSlider, HueSlider};
use super::swatches::SwatchesGrid;
use super::types::{parse_color_value, ColorFormat, SwatchItem, TriggerMode};
use crate::primitives::tokens::Size;
use leptos::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[component]
pub fn ColorPicker(
    #[prop(into, default = None)] value: Option<Signal<String>>,
    #[prop(optional, default = "#000000")] default_value: &'static str,
    #[prop(optional)] format: ColorFormat,
    #[prop(optional)] alpha: bool,
    #[prop(optional, default = vec![])] swatches: Vec<SwatchItem>,
    #[prop(optional)] trigger_mode: TriggerMode,
    #[prop(optional)] size: Size,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    #[prop(into, default = None)] on_change_end: Option<Callback<String>>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if size == Size::Lg {
        web_sys::console::warn_1(
            &"ColorPicker: Size::Lg is not supported. Using Md.".into(),
        );
    }

    let initial_hsv = parse_color_value(default_value).unwrap_or_default();
    let hsv = RwSignal::new(initial_hsv);
    let active_format = RwSignal::new(format);

    // Controlled mode: sync external value -> internal HSV
    if let Some(ext) = value {
        Effect::new(move || {
            let val = ext.get();
            if let Some(parsed) = parse_color_value(&val) {
                let current = hsv.get_untracked();
                if (parsed.h - current.h).abs() > 0.5
                    || (parsed.s - current.s).abs() > 0.5
                    || (parsed.v - current.v).abs() > 0.5
                    || (parsed.a - current.a).abs() > 0.5
                {
                    hsv.set(parsed);
                }
            }
        });
    }

    // Fire on_change when HSV changes
    if let Some(cb) = on_change {
        Effect::new(move || {
            let output = hsv.get().format_output(format, alpha);
            cb.run(output);
        });
    }

    // Panel state
    let mounted = RwSignal::new(false);
    let data_state = RwSignal::new(Option::<&'static str>::None);
    let panel_style = RwSignal::new(String::new());

    let trigger_ref = NodeRef::<leptos::html::Div>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();

    // Version counters for race-safe open/close
    let show_ver = Arc::new(AtomicU32::new(0));
    let hide_ver = Arc::new(AtomicU32::new(0));

    // ---- Close (as Callback — Copy, usable in multiple closures) ----
    let close_sv = show_ver.clone();
    let close_hv = hide_ver.clone();
    let do_close = Callback::new(move |()| {
        close_sv.fetch_add(1, Ordering::Relaxed);
        data_state.set(Some("closed"));

        let ver = close_hv.load(Ordering::Relaxed);
        let hv = close_hv.clone();
        set_timeout(
            move || {
                if hv.load(Ordering::Relaxed) == ver {
                    mounted.set(false);
                    data_state.set(None);
                }
            },
            Duration::from_millis(100),
        );

        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    });

    // ---- Open (as Callback) ----
    let swatches_empty = swatches.is_empty();
    let open_sv = show_ver.clone();
    let open_hv = hide_ver.clone();
    let do_open = Callback::new(move |()| {
        if disabled {
            return;
        }
        open_hv.fetch_add(1, Ordering::Relaxed);

        // Position calculation
        if let Some(el) = trigger_ref.get() {
            let rect = el.get_bounding_client_rect();
            let viewport_height = web_sys::window()
                .and_then(|w| w.inner_height().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0);

            let panel_h = 220.0
                + if alpha { 18.0 } else { 0.0 }
                + if !swatches_empty { 30.0 } else { 0.0 };

            let space_below = viewport_height - rect.bottom() - 8.0;
            let place_below = space_below >= panel_h || rect.top() < panel_h + 8.0;

            let top = if place_below {
                rect.bottom() + 8.0
            } else {
                rect.top() - panel_h - 8.0
            };

            panel_style.set(format!("left: {}px; top: {}px;", rect.left(), top));
        }

        data_state.set(None);
        mounted.set(true);

        let ver = open_sv.load(Ordering::Relaxed);
        let sv2 = open_sv.clone();
        set_timeout(
            move || {
                if sv2.load(Ordering::Relaxed) == ver {
                    data_state.set(Some("open"));
                }
            },
            Duration::ZERO,
        );

        // Focus gradient area after panel mounts
        set_timeout(
            move || {
                if let Some(panel) = panel_ref.get() {
                    if let Ok(Some(gradient)) = panel.query_selector(".cp-gradient") {
                        if let Some(el) = gradient.dyn_ref::<web_sys::HtmlElement>() {
                            let _ = el.focus();
                        }
                    }
                }
            },
            Duration::from_millis(10),
        );
    });

    // ---- Trigger class ----
    let is_open = move || mounted.get();

    let trigger_cls = move || {
        match trigger_mode {
            TriggerMode::SwatchInput => [
                "color-picker-trigger",
                if size == Size::Sm {
                    "color-picker-trigger--sm"
                } else {
                    ""
                },
                if is_open() {
                    "color-picker-trigger--open"
                } else {
                    ""
                },
                if disabled {
                    "color-picker-trigger--disabled"
                } else {
                    ""
                },
                class,
            ]
            .join(" "),
            TriggerMode::SwatchOnly => [
                "color-picker-trigger-only",
                if size == Size::Sm {
                    "color-picker-trigger-only--sm"
                } else {
                    ""
                },
                if is_open() {
                    "color-picker-trigger-only--open"
                } else {
                    ""
                },
                if disabled {
                    "color-picker-trigger-only--disabled"
                } else {
                    ""
                },
                class,
            ]
            .join(" "),
        }
    };

    let swatch_bg = move || {
        let c = hsv.get();
        let (r, g, b) = c.to_rgb();
        if alpha && c.a < 100.0 {
            format!(
                "background-color: rgba({}, {}, {}, {})",
                r,
                g,
                b,
                c.a / 100.0
            )
        } else {
            format!("background-color: rgb({}, {}, {})", r, g, b)
        }
    };

    let swatch_class = move || {
        if alpha && hsv.get().a < 100.0 {
            "color-picker-trigger__swatch cp-checkerboard"
        } else {
            "color-picker-trigger__swatch"
        }
    };

    let swatch_only_class = move || {
        if alpha && hsv.get().a < 100.0 {
            "color-picker-trigger-only__swatch cp-checkerboard"
        } else {
            "color-picker-trigger-only__swatch"
        }
    };

    let display_text = move || hsv.get().to_hex(alpha);

    view! {
        <div
            style="position: relative; display: inline-block;"
            style:width=move || {
                if trigger_mode == TriggerMode::SwatchOnly { "auto" } else { "100%" }
            }
        >
            // Trigger
            <div
                node_ref=trigger_ref
                class=trigger_cls
                role="button"
                tabindex=if disabled { -1 } else { 0 }
                aria-haspopup="dialog"
                aria-expanded=move || is_open().to_string()
                aria-label="Color picker"
                on:click=move |_: web_sys::MouseEvent| {
                    if disabled {
                        return;
                    }
                    if mounted.get_untracked() {
                        do_close.run(());
                    } else {
                        do_open.run(());
                    }
                }
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" || ev.key() == " " {
                        ev.prevent_default();
                        if disabled {
                            return;
                        }
                        if mounted.get_untracked() {
                            do_close.run(());
                        } else {
                            do_open.run(());
                        }
                    }
                }
            >
                {match trigger_mode {
                    TriggerMode::SwatchInput => {
                        view! {
                            <div class=swatch_class style=swatch_bg />
                            <span class="color-picker-trigger__text">{display_text}</span>
                            <span class="color-picker-trigger__chevron">
                                <svg
                                    viewBox="0 0 16 16"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path d="M4 6l4 4 4-4" />
                                </svg>
                            </span>
                        }
                            .into_any()
                    }
                    TriggerMode::SwatchOnly => {
                        view! { <div class=swatch_only_class style=swatch_bg /> }.into_any()
                    }
                }}
            </div>

            // Panel: backdrop + floating panel
            <Show when=move || mounted.get()>
                // Invisible backdrop catches clicks outside the panel
                <div
                    style="position: fixed; inset: 0; z-index: 49;"
                    on:mousedown=move |_: web_sys::MouseEvent| do_close.run(())
                />
                <div
                    node_ref=panel_ref
                    class="color-picker-panel"
                    style=move || panel_style.get()
                    role="dialog"
                    aria-label="Color picker"
                    data-state=move || data_state.get()
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            ev.prevent_default();
                            ev.stop_propagation();
                            do_close.run(());
                        }
                    }
                >
                    <Gradient hsv=hsv on_change_end=on_change_end format=format alpha=alpha />
                    <HueSlider hsv=hsv />
                    {alpha.then(move || view! { <AlphaSlider hsv=hsv /> })}
                    <div class="cp-divider"></div>
                    <FormatRow
                        hsv=hsv
                        active_format=active_format
                        alpha=alpha
                        on_change_end=on_change_end
                        format=format
                    />
                    <SwatchesGrid
                        swatches=swatches.clone()
                        hsv=hsv
                        on_change_end=on_change_end
                        format=format
                        alpha=alpha
                    />
                </div>
            </Show>
        </div>
    }
}
