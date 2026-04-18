use super::types::{flatten_options, SelectItem};
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Size, Status};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[component]
pub fn Select(
    options: Vec<SelectItem>,
    #[prop(into, default = None)] value: Option<Signal<String>>,
    #[prop(optional, default = "")] default_value: &'static str,
    #[prop(into, default = TextProp::default())] placeholder: TextProp,
    #[prop(optional)] size: Size,
    #[prop(optional)] status: Status,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_change: Option<Callback<String>>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let internal = RwSignal::new(default_value.to_string());
    let selected = Memo::new(move |_| value.map(|s| s.get()).unwrap_or_else(|| internal.get()));

    // Flatten for keyboard navigation
    let flat_options = StoredValue::new(flatten_options(&options));
    let items = StoredValue::new(options);

    // Panel state (same pattern as ColorPicker)
    let mounted = RwSignal::new(false);
    let data_state = RwSignal::new(String::new());
    let panel_style = RwSignal::new(String::new());
    let highlighted = RwSignal::new(Option::<usize>::None);

    let trigger_ref = NodeRef::<leptos::html::Div>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();

    // Type-ahead state
    let type_buffer = StoredValue::new(String::new());
    let type_timestamp = StoredValue::new(0.0_f64);

    // Version counters for race-safe open/close
    let show_ver = Arc::new(AtomicU32::new(0));
    let hide_ver = Arc::new(AtomicU32::new(0));

    // ---- Close ----
    let close_sv = show_ver.clone();
    let close_hv = hide_ver.clone();
    let do_close = Callback::new(move |()| {
        close_sv.fetch_add(1, Ordering::Relaxed);
        data_state.set("closed".into());

        let ver = close_hv.load(Ordering::Relaxed);
        let hv = close_hv.clone();
        set_timeout(
            move || {
                if hv.load(Ordering::Relaxed) == ver {
                    mounted.set(false);
                    data_state.set(String::new());
                }
            },
            Duration::from_millis(100),
        );

        highlighted.set(None);

        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    });

    // ---- Open ----
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

            let max_panel_h = 240.0;
            let space_below = viewport_height - rect.bottom() - 8.0;
            let place_below = space_below >= max_panel_h || rect.top() < max_panel_h + 8.0;

            if place_below {
                // Anchor to top of panel = bottom of trigger + gap
                panel_style.set(format!(
                    "left: {}px; top: {}px; min-width: {}px;",
                    rect.left(),
                    rect.bottom() + 4.0,
                    rect.width()
                ));
            } else {
                // Anchor to bottom of panel = top of trigger - gap
                // Use `bottom` instead of calculating from panel height
                panel_style.set(format!(
                    "left: {}px; bottom: {}px; min-width: {}px;",
                    rect.left(),
                    viewport_height - rect.top() + 4.0,
                    rect.width()
                ));
            }
        }

        // Highlight current selection
        let sel = selected.get_untracked();
        let flat = flat_options.get_value();
        let idx = flat.iter().position(|o| o.value == sel);
        highlighted.set(idx);

        data_state.set(String::new());
        mounted.set(true);

        let ver = open_sv.load(Ordering::Relaxed);
        let sv2 = open_sv.clone();
        set_timeout(
            move || {
                if sv2.load(Ordering::Relaxed) == ver {
                    data_state.set("open".into());
                }
            },
            Duration::ZERO,
        );

        // Focus the panel
        set_timeout(
            move || {
                if let Some(panel) = panel_ref.get() {
                    let raw: &web_sys::HtmlElement = &panel;
                    let _ = raw.focus();
                }
            },
            Duration::from_millis(10),
        );
    });

    // ---- Select a value ----
    let select_value = move |val: String| {
        internal.set(val.clone());
        if let Some(cb) = on_change {
            cb.run(val);
        }
        do_close.run(());
    };

    // ---- Find label for current value ----
    let display_text = move || {
        let sel = selected.get();
        if sel.is_empty() {
            return None;
        }
        let flat = flat_options.get_value();
        flat.iter().find(|o| o.value == sel).map(|o| o.label.clone())
    };

    let is_open = move || mounted.get();

    // ---- Trigger class ----
    let trigger_cls = move || {
        [
            "select-trigger",
            size.select_trigger_class(),
            status.select_trigger_class(),
            if is_open() { "select-trigger--open" } else { "" },
            if disabled {
                "select-trigger--disabled"
            } else {
                ""
            },
            class,
        ]
        .join(" ")
    };

    // ---- Trigger keyboard ----
    let handle_trigger_keydown = move |ev: web_sys::KeyboardEvent| {
        match ev.key().as_str() {
            "Enter" | " " | "ArrowDown" | "ArrowUp" => {
                ev.prevent_default();
                if !mounted.get_untracked() {
                    do_open.run(());
                }
            }
            _ => {}
        }
    };

    // ---- Panel keyboard ----
    let handle_panel_keydown = move |ev: web_sys::KeyboardEvent| {
        let flat = flat_options.get_value();
        let len = flat.len();
        if len == 0 {
            return;
        }

        let key = ev.key();
        match key.as_str() {
            "Escape" => {
                ev.prevent_default();
                ev.stop_propagation();
                do_close.run(());
            }
            "ArrowDown" => {
                ev.prevent_default();
                let cur = highlighted.get_untracked().unwrap_or(len.wrapping_sub(1));
                let mut next = cur;
                for _ in 0..len {
                    next = (next + 1) % len;
                    if !flat[next].disabled {
                        break;
                    }
                }
                highlighted.set(Some(next));
                scroll_option_into_view(panel_ref, next);
            }
            "ArrowUp" => {
                ev.prevent_default();
                let cur = highlighted.get_untracked().unwrap_or(0);
                let mut next = cur;
                for _ in 0..len {
                    next = if next == 0 { len - 1 } else { next - 1 };
                    if !flat[next].disabled {
                        break;
                    }
                }
                highlighted.set(Some(next));
                scroll_option_into_view(panel_ref, next);
            }
            "Home" => {
                ev.prevent_default();
                if let Some(idx) = flat.iter().position(|o| !o.disabled) {
                    highlighted.set(Some(idx));
                    scroll_option_into_view(panel_ref, idx);
                }
            }
            "End" => {
                ev.prevent_default();
                if let Some(idx) = flat.iter().rposition(|o| !o.disabled) {
                    highlighted.set(Some(idx));
                    scroll_option_into_view(panel_ref, idx);
                }
            }
            "Enter" | " " => {
                ev.prevent_default();
                if let Some(idx) = highlighted.get_untracked() {
                    if idx < len && !flat[idx].disabled {
                        select_value(flat[idx].value.clone());
                    }
                }
            }
            _ => {
                // Type-ahead
                if key.len() == 1 {
                    let now = js_sys::Date::now();
                    let prev_time = type_timestamp.get_value();
                    if now - prev_time > 300.0 {
                        type_buffer.set_value(String::new());
                    }
                    type_timestamp.set_value(now);

                    let mut buf = type_buffer.get_value();
                    buf.push_str(&key.to_lowercase());
                    type_buffer.set_value(buf.clone());

                    if let Some(idx) = flat
                        .iter()
                        .position(|o| !o.disabled && o.label.to_lowercase().starts_with(&buf))
                    {
                        highlighted.set(Some(idx));
                        scroll_option_into_view(panel_ref, idx);
                    }
                }
            }
        }
    };

    // ---- Build option views (rendered inside Show) ----
    let render_items = move || {
        items
            .get_value()
            .into_iter()
            .map(|item| render_select_item(item, flat_options, highlighted, selected, select_value))
            .collect::<Vec<_>>()
    };

    view! {
        <div style="position: relative; display: inline-block; width: 100%;">
            // Trigger
            <div
                node_ref=trigger_ref
                class=trigger_cls
                role="combobox"
                tabindex=if disabled { -1 } else { 0 }
                aria-haspopup="listbox"
                aria-expanded=move || is_open().to_string()
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
                on:keydown=handle_trigger_keydown
            >
                {move || {
                    if let Some(text) = display_text() {
                        view! { <span class="select-trigger__text">{text}</span> }.into_any()
                    } else {
                        view! {
                            <span class="select-trigger__text select-trigger__placeholder">
                                {move || placeholder.get()}
                            </span>
                        }
                            .into_any()
                    }
                }}
                <span class="select-trigger__chevron">
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
            </div>

            // Dropdown panel
            <Show when=move || mounted.get()>
                // Invisible backdrop
                <div
                    style="position: fixed; inset: 0; z-index: 49;"
                    on:mousedown=move |_: web_sys::MouseEvent| do_close.run(())
                />
                <div
                    node_ref=panel_ref
                    class="select-panel"
                    style=move || panel_style.get()
                    role="listbox"
                    tabindex="-1"
                    attr:data-state=move || {
                        let s = data_state.get();
                        if s.is_empty() { None } else { Some(s) }
                    }
                    on:keydown=handle_panel_keydown
                >
                    {render_items}
                </div>
            </Show>
        </div>
    }
}

fn render_select_item(
    item: SelectItem,
    flat_options: StoredValue<Vec<super::types::SelectOption>>,
    highlighted: RwSignal<Option<usize>>,
    selected: Memo<String>,
    select_value: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    match item {
        SelectItem::Option(opt) => {
            render_option(opt, flat_options, highlighted, selected, select_value).into_any()
        }
        SelectItem::Group(group) => {
            let label = group.label.clone();
            view! {
                <div class="select-group" role="group">
                    <div class="select-group__label" role="presentation">
                        {label}
                    </div>
                    {group
                        .options
                        .into_iter()
                        .map(|opt| render_option(
                            opt,
                            flat_options,
                            highlighted,
                            selected,
                            select_value,
                        ))
                        .collect::<Vec<_>>()}
                </div>
            }
            .into_any()
        }
    }
}

fn render_option(
    opt: super::types::SelectOption,
    flat_options: StoredValue<Vec<super::types::SelectOption>>,
    highlighted: RwSignal<Option<usize>>,
    selected: Memo<String>,
    select_value: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    let val = opt.value.clone();
    let val2 = opt.value.clone();
    let label = opt.label.clone();
    let opt_disabled = opt.disabled;

    // Find this option's flat index
    let flat_idx = flat_options
        .get_value()
        .iter()
        .position(|o| o.value == val)
        .unwrap_or(0);

    let is_selected_cls = {
        let v = val2.clone();
        move || selected.get() == v
    };
    let is_selected_aria = {
        let v = val2.clone();
        move || selected.get() == v
    };
    let is_selected_icon = move || selected.get() == val2;
    let is_highlighted = move || highlighted.get() == Some(flat_idx);

    let option_cls = move || {
        let mut cls = String::from("select-option");
        if is_selected_cls() {
            cls.push_str(" select-option--selected");
        }
        if is_highlighted() {
            cls.push_str(" select-option--highlighted");
        }
        if opt_disabled {
            cls.push_str(" select-option--disabled");
        }
        cls
    };

    let val_click = opt.value.clone();
    let on_click = move |_: web_sys::MouseEvent| {
        if !opt_disabled {
            select_value(val_click.clone());
        }
    };

    let on_mouseenter = move |_: web_sys::MouseEvent| {
        if !opt_disabled {
            highlighted.set(Some(flat_idx));
        }
    };

    view! {
        <div
            class=option_cls
            role="option"
            aria-selected=move || is_selected_aria().to_string()
            aria-disabled=if opt_disabled { Some("true") } else { None }
            on:click=on_click
            on:mouseenter=on_mouseenter
        >
            <span class="select-option__text">{label}</span>
            {move || {
                if is_selected_icon() {
                    Some(
                        view! {
                            <svg
                                class="select-option__check"
                                viewBox="0 0 16 16"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="3.5,8 6.5,11 12.5,5" />
                            </svg>
                        },
                    )
                } else {
                    None
                }
            }}
        </div>
    }
}

fn scroll_option_into_view(
    panel_ref: NodeRef<leptos::html::Div>,
    idx: usize,
) {
    if let Some(panel) = panel_ref.get() {
        let raw: &web_sys::HtmlElement = &panel;
        if let Ok(list) = raw.query_selector_all("[role=\"option\"]") {
            if let Some(node) = list.item(idx as u32) {
                if let Some(el) = node.dyn_ref::<web_sys::HtmlElement>() {
                    el.scroll_into_view_with_bool(false);
                }
            }
        }
    }
}
