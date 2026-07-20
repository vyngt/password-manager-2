use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{ProgressVariant, Size};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

static PROGRESS_ID_SEQ: AtomicU64 = AtomicU64::new(0);

#[component]
pub fn ProgressBar(
    #[prop(into)] value: Signal<f64>,
    #[prop(optional)] variant: ProgressVariant,
    #[prop(optional)] size: Size,
    #[prop(into, default = TextProp::default())] label: TextProp,
    #[prop(optional)] show_value: bool,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let seq = PROGRESS_ID_SEQ.fetch_add(1, Ordering::Relaxed);
    let label_id = StoredValue::new(format!("progress-label-{}", seq));

    let clamped = Memo::new(move |_| {
        let v = value.get();
        #[cfg(debug_assertions)]
        if !(0.0..=100.0).contains(&v) {
            web_sys::console::warn_1(
                &format!("ProgressBar: value {} is outside 0–100 and was clamped.", v).into(),
            );
        }
        v.clamp(0.0, 100.0)
    });

    // Enable the width transition only after the first render so the initial
    // paint does not animate from 0 to the starting value.
    let animated = RwSignal::new(false);
    Effect::new(move |_| {
        animated.set(true);
    });

    let root_cls = ["progress", class].join(" ");
    let track_cls = ["progress__track", size.progress_track_class()].join(" ");

    let fill_cls = move || {
        let mut c = String::from("progress__fill");
        let variant_cls = variant.progress_fill_class();
        if !variant_cls.is_empty() {
            c.push(' ');
            c.push_str(variant_cls);
        }
        if animated.get() {
            c.push_str(" progress__fill--animated");
        }
        c
    };

    let fill_style = move || format!("width: {}%", clamped.get());
    let value_rounded = move || format!("{}%", clamped.get().round() as i64);

    let has_visible_label = Memo::new(move |_| !label.get().is_empty());
    let show_header = move || has_visible_label.get() || show_value;

    let header = move || {
        show_header().then(|| {
            view! {
                <div class="progress__header">
                    <Show
                        when=move || has_visible_label.get()
                        fallback=|| view! { <span class="progress__label"></span> }
                    >
                        <span class="progress__label" id=label_id.get_value()>
                            {move || label.get()}
                        </span>
                    </Show>
                    {move || {
                        show_value
                            .then(|| {
                                view! { <span class="progress__value">{value_rounded()}</span> }
                            })
                    }}
                </div>
            }
        })
    };

    // aria-labelledby when visible label is present; otherwise aria-label.
    let aria_labelledby_attr = move || {
        if has_visible_label.get() {
            Some(label_id.get_value())
        } else {
            None
        }
    };
    let aria_label_attr = move || {
        if has_visible_label.get() {
            None
        } else {
            let v = aria_label.get();
            if v.is_empty() { None } else { Some(v) }
        }
    };

    view! {
        <div class=root_cls>
            {header}
            <div
                class=track_cls
                role="progressbar"
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow=move || clamped.get().round().to_string()
                aria-label=aria_label_attr
                aria-labelledby=aria_labelledby_attr
            >
                <div class=fill_cls style=fill_style></div>
            </div>
        </div>
    }
}
