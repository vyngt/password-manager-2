use crate::primitives::tokens::{Size, Status};
use leptos::prelude::*;
use web_sys::HtmlTextAreaElement;

#[component]
pub fn Textarea(
    id: &'static str,
    #[prop(into, default = None)] value: Option<Signal<String>>,
    #[prop(optional, default = "")] default_value: &'static str,
    #[prop(into, default = None)] placeholder: Option<Signal<String>>,
    #[prop(optional, default = 3)] rows: u32,
    #[prop(optional, default = 10)] max_rows: u32,
    #[prop(optional)] size: Size,
    #[prop(optional)] status: Status,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] read_only: bool,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    /// When the signal is `true`, masks the content like a password field
    /// (`-webkit-text-security`) — e.g. an SSH PEM block behind a reveal toggle.
    #[prop(into, default = None)]
    masked: Option<Signal<bool>>,
    #[prop(optional, default = "")] class: &'static str,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(optional)] required: bool,
) -> impl IntoView {
    let internal = RwSignal::new(default_value.to_string());
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Line-height and padding per size (must match CSS exactly)
    let line_height: f64 = match size {
        Size::Xs | Size::Sm => 16.0,
        Size::Md => 20.0,
        Size::Lg => 24.0,
    };
    let padding_y: f64 = match size {
        Size::Xs | Size::Sm => 6.0,
        Size::Md => 8.0,
        Size::Lg => 10.0,
    };
    let border: f64 = 2.0; // 1px top + 1px bottom

    let min_height = (rows as f64) * line_height + padding_y * 2.0 + border;
    let max_height = (max_rows as f64) * line_height + padding_y * 2.0 + border;

    let auto_resize = move || {
        // Imperative DOM work — often deferred via `request_animation_frame`
        // (no reactive owner), so read the ref untracked to avoid the
        // "signal accessed outside a reactive tracking context" warning.
        if let Some(el) = textarea_ref.get_untracked() {
            let raw: &HtmlTextAreaElement = &el;
            // Reset height to auto so scrollHeight reflects true content height
            let _ = raw.set_attribute(
                "style",
                &format!("min-height:{min_height}px;max-height:{max_height}px;height:auto"),
            );
            let scroll_h = raw.scroll_height() as f64;
            let clamped = scroll_h.max(min_height).min(max_height);
            let _ = raw.set_attribute(
                "style",
                &format!("min-height:{min_height}px;max-height:{max_height}px;height:{clamped}px"),
            );
        }
    };

    let handle_input = move |_: web_sys::Event| {
        if let Some(el) = textarea_ref.get() {
            let raw: &HtmlTextAreaElement = &el;
            let val = raw.value();
            internal.set(val.clone());
            if let Some(cb) = on_change {
                cb.run(val);
            }
        }
        auto_resize();
    };

    // Resize on mount and when controlled value changes
    Effect::new(move || {
        let _val = value.map(|s| s.get());
        request_animation_frame(move || {
            auto_resize();
        });
    });

    // CSS class
    let root_cls = [
        "textarea",
        size.textarea_class(),
        status.textarea_class(),
        if disabled { "textarea--disabled" } else { "" },
        class,
    ]
    .join(" ");

    // ARIA
    let aria_invalid = if status == Status::Error {
        Some("true")
    } else {
        None
    };
    let aria_describedby_attr = if aria_describedby.is_empty() {
        None
    } else {
        Some(aria_describedby)
    };
    let aria_required_attr = if required { Some("true") } else { None };

    let style_str = format!("min-height: {min_height}px; max-height: {max_height}px;");

    view! {
        <textarea
            node_ref=textarea_ref
            class=root_cls
            class=("textarea--masked", move || masked.map(|m| m.get()).unwrap_or(false))
            id=id
            disabled=disabled
            readonly=read_only
            rows=rows
            placeholder=move || placeholder.map(|s| s.get()).unwrap_or_default()
            prop:value=move || value.map(|s| s.get()).unwrap_or_else(|| internal.get())
            style=style_str
            aria-invalid=aria_invalid
            aria-describedby=aria_describedby_attr
            aria-required=aria_required_attr
            on:input=handle_input
        />
    }
}
