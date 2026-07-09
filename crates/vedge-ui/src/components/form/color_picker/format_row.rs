use super::types::{ColorFormat, HsvColor, hsl_to_hsv, hsv_to_hsl, hsv_to_rgb, rgb_to_hsv};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub(super) fn FormatRow(
    hsv: RwSignal<HsvColor>,
    active_format: RwSignal<ColorFormat>,
    alpha: bool,
    #[prop(into, default = None)] on_change_end: Option<Callback<String>>,
    format: ColorFormat,
) -> impl IntoView {
    let fire_change_end = move || {
        if let Some(cb) = on_change_end {
            cb.run(hsv.get_untracked().format_output(format, alpha));
        }
    };

    // Check EyeDropper API availability
    let eyedropper_supported = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"EyeDropper".into()).ok())
        .is_some_and(|v| v.is_function());

    let on_eyedropper = move |_: web_sys::MouseEvent| {
        let fire = fire_change_end;
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(hex) = pick_from_screen().await {
                if let Some(parsed) = HsvColor::from_hex(&hex) {
                    hsv.set(parsed);
                    fire();
                }
            }
        });
    };

    view! {
        <div class="cp-format-row">
            <div class="cp-format-controls">
                {eyedropper_supported
                    .then(move || {
                        view! {
                            <button
                                type="button"
                                class="cp-eyedropper"
                                aria-label="Pick color from screen"
                                on:click=on_eyedropper
                            >
                                <svg
                                    viewBox="0 0 16 16"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path d="M13.5 2.5a1.8 1.8 0 0 0-2.5 0L9.2 4.3 7.5 2.5l-1 1 1.8 1.8-5 5-.3 2.2 2.2-.3 5-5 1.8 1.8 1-1-1.8-1.8 1.8-1.8a1.8 1.8 0 0 0 0-2.5Z" />
                                </svg>
                            </button>
                        }
                    })} <FormatSwitcher active_format=active_format />
            </div>

            <FormatInputs
                hsv=hsv
                active_format=active_format
                alpha=alpha
                on_commit=Callback::new(move |(): ()| fire_change_end())
            />
        </div>
    }
}

/// Mini segmented control for HEX / RGB / HSL.
#[component]
fn FormatSwitcher(active_format: RwSignal<ColorFormat>) -> impl IntoView {
    let formats = [
        (ColorFormat::Hex, "HEX"),
        (ColorFormat::Rgb, "RGB"),
        (ColorFormat::Hsl, "HSL"),
    ];

    view! {
        <div class="cp-format-switcher">
            {formats
                .into_iter()
                .map(|(fmt, label)| {
                    let is_active = move || active_format.get() == fmt;
                    let cls = move || {
                        if is_active() {
                            "cp-format-btn cp-format-btn--active"
                        } else {
                            "cp-format-btn"
                        }
                    };
                    view! {
                        <button
                            type="button"
                            class=cls
                            on:click=move |_| active_format.set(fmt)
                            aria-pressed=move || is_active().to_string()
                        >
                            {label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Renders the appropriate input fields based on active format.
#[component]
fn FormatInputs(
    hsv: RwSignal<HsvColor>,
    active_format: RwSignal<ColorFormat>,
    alpha: bool,
    on_commit: Callback<()>,
) -> impl IntoView {
    view! {
        {move || match active_format.get() {
            ColorFormat::Hex => {
                view! { <HexInput hsv=hsv alpha=alpha on_commit=on_commit /> }.into_any()
            }
            ColorFormat::Rgb => {
                view! { <RgbInputs hsv=hsv alpha=alpha on_commit=on_commit /> }.into_any()
            }
            ColorFormat::Hsl => {
                view! { <HslInputs hsv=hsv alpha=alpha on_commit=on_commit /> }.into_any()
            }
        }}
    }
}

/// Single hex input field.
#[component]
fn HexInput(hsv: RwSignal<HsvColor>, alpha: bool, on_commit: Callback<()>) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let has_error = RwSignal::new(false);

    // Sync text from HSV when HSV changes externally
    Effect::new(move || {
        let hex = hsv.get().to_hex(alpha);
        text.set(hex);
        has_error.set(false);
    });

    let commit = move || {
        let val = text.get_untracked();
        if let Some(parsed) = HsvColor::from_hex(&val) {
            hsv.set(if alpha {
                parsed
            } else {
                HsvColor { a: 100.0, ..parsed }
            });
            has_error.set(false);
            on_commit.run(());
        } else {
            has_error.set(true);
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            commit();
        }
    };

    let cls = move || {
        if has_error.get() {
            "cp-input cp-input--hex cp-input--error"
        } else {
            "cp-input cp-input--hex"
        }
    };

    view! {
        <div class="cp-input-group">
            <div class="cp-input-wrapper">
                <input
                    type="text"
                    class=cls
                    prop:value=move || text.get()
                    on:input=move |ev| {
                        let target = event_target::<web_sys::HtmlInputElement>(&ev);
                        text.set(target.value());
                        has_error.set(false);
                    }
                    on:blur=move |_| commit()
                    on:keydown=on_keydown
                    aria-label="Hex color"
                    spellcheck="false"
                    autocomplete="off"
                />
                <span class="cp-input-label">{"HEX"}</span>
            </div>
        </div>
    }
}

/// RGB channel inputs.
#[component]
fn RgbInputs(hsv: RwSignal<HsvColor>, alpha: bool, on_commit: Callback<()>) -> impl IntoView {
    let r_text = RwSignal::new(String::new());
    let g_text = RwSignal::new(String::new());
    let b_text = RwSignal::new(String::new());
    let a_text = RwSignal::new(String::new());

    // Sync from HSV
    Effect::new(move || {
        let c = hsv.get();
        let (r, g, b) = hsv_to_rgb(c.h, c.s, c.v);
        r_text.set(r.to_string());
        g_text.set(g.to_string());
        b_text.set(b.to_string());
        a_text.set(c.a.round().to_string());
    });

    let commit_rgb = move || {
        let r: Option<u8> = r_text.get_untracked().parse().ok();
        let g: Option<u8> = g_text.get_untracked().parse().ok();
        let b: Option<u8> = b_text.get_untracked().parse().ok();
        if let (Some(r), Some(g), Some(b)) = (r, g, b) {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            let a = if alpha {
                a_text
                    .get_untracked()
                    .parse::<f64>()
                    .unwrap_or(100.0)
                    .clamp(0.0, 100.0)
            } else {
                100.0
            };
            hsv.set(HsvColor::new(h, s, v, a));
            on_commit.run(());
        }
    };

    view! {
        <div class="cp-input-group">
            <ChannelInput text=r_text label="Red" on_commit=move || commit_rgb() />
            <ChannelInput text=g_text label="Green" on_commit=move || commit_rgb() />
            <ChannelInput text=b_text label="Blue" on_commit=move || commit_rgb() />
            {alpha
                .then(move || {
                    view! {
                        <ChannelInput text=a_text label="Alpha" on_commit=move || commit_rgb() />
                    }
                })}
        </div>
    }
}

/// HSL channel inputs.
#[component]
fn HslInputs(hsv: RwSignal<HsvColor>, alpha: bool, on_commit: Callback<()>) -> impl IntoView {
    let h_text = RwSignal::new(String::new());
    let s_text = RwSignal::new(String::new());
    let l_text = RwSignal::new(String::new());
    let a_text = RwSignal::new(String::new());

    // Sync from HSV
    Effect::new(move || {
        let c = hsv.get();
        let (h, s, l) = hsv_to_hsl(c.h, c.s, c.v);
        h_text.set((h.round() as i32).to_string());
        s_text.set((s.round() as i32).to_string());
        l_text.set((l.round() as i32).to_string());
        a_text.set(c.a.round().to_string());
    });

    let commit_hsl = move || {
        let h: Option<f64> = h_text.get_untracked().parse().ok();
        let s: Option<f64> = s_text.get_untracked().parse().ok();
        let l: Option<f64> = l_text.get_untracked().parse().ok();
        if let (Some(h), Some(s), Some(l)) = (h, s, l) {
            let (h, sv_s, sv_v) = hsl_to_hsv(
                h.clamp(0.0, 360.0),
                s.clamp(0.0, 100.0),
                l.clamp(0.0, 100.0),
            );
            let a = if alpha {
                a_text
                    .get_untracked()
                    .parse::<f64>()
                    .unwrap_or(100.0)
                    .clamp(0.0, 100.0)
            } else {
                100.0
            };
            hsv.set(HsvColor::new(h, sv_s, sv_v, a));
            on_commit.run(());
        }
    };

    view! {
        <div class="cp-input-group">
            <ChannelInput text=h_text label="Hue" on_commit=move || commit_hsl() />
            <ChannelInput text=s_text label="Saturation" on_commit=move || commit_hsl() />
            <ChannelInput text=l_text label="Lightness" on_commit=move || commit_hsl() />
            {alpha
                .then(move || {
                    view! {
                        <ChannelInput text=a_text label="Alpha" on_commit=move || commit_hsl() />
                    }
                })}
        </div>
    }
}

/// Reusable single-channel numeric input.
#[component]
fn ChannelInput(
    text: RwSignal<String>,
    label: &'static str,
    on_commit: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let short_label = match label {
        "Red" => "R",
        "Green" => "G",
        "Blue" => "B",
        "Alpha" => "A",
        "Hue" => "H",
        "Saturation" => "S",
        "Lightness" => "L",
        other => other,
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            on_commit();
        }
    };

    view! {
        <div class="cp-input-wrapper">
            <input
                type="text"
                class="cp-input"
                prop:value=move || text.get()
                on:input=move |ev| {
                    let target = event_target::<web_sys::HtmlInputElement>(&ev);
                    text.set(target.value());
                }
                on:blur=move |_| on_commit()
                on:keydown=on_keydown
                aria-label=label
                spellcheck="false"
                autocomplete="off"
                inputmode="numeric"
            />
            <span class="cp-input-label">{short_label}</span>
        </div>
    }
}

// ---------------------------------------------------------------------------
// EyeDropper API (progressive enhancement)
// ---------------------------------------------------------------------------

async fn pick_from_screen() -> Option<String> {
    let window = web_sys::window()?;
    let constructor = js_sys::Reflect::get(&window, &"EyeDropper".into()).ok()?;
    let constructor_fn: &js_sys::Function = constructor.dyn_ref()?;
    let instance = js_sys::Reflect::construct(constructor_fn, &js_sys::Array::new()).ok()?;
    let open_fn: js_sys::Function = js_sys::Reflect::get(&instance, &"open".into())
        .ok()?
        .dyn_into()
        .ok()?;
    let promise: js_sys::Promise = open_fn.call0(&instance).ok()?.dyn_into().ok()?;
    let result = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    js_sys::Reflect::get(&result, &"sRGBHex".into())
        .ok()?
        .as_string()
}
