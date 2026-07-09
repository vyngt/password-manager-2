use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

const FOCUSABLE_SELECTOR: &str = "button:not([disabled]), [href], input:not([disabled]), \
    select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/// Preferred panel placement. Popover auto-flips to the opposite primary axis
/// when the preferred side would overflow the viewport; alignment is preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopoverPlacement {
    Top,
    TopStart,
    TopEnd,
    Bottom,
    #[default]
    BottomStart,
    BottomEnd,
    Left,
    LeftStart,
    LeftEnd,
    Right,
    RightStart,
    RightEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Start,
    Center,
    End,
}

impl PopoverPlacement {
    fn parts(self) -> (Side, Align) {
        use PopoverPlacement::{
            Bottom, BottomEnd, BottomStart, Left, LeftEnd, LeftStart, Right, RightEnd, RightStart,
            Top, TopEnd, TopStart,
        };
        match self {
            Top => (Side::Top, Align::Center),
            TopStart => (Side::Top, Align::Start),
            TopEnd => (Side::Top, Align::End),
            Bottom => (Side::Bottom, Align::Center),
            BottomStart => (Side::Bottom, Align::Start),
            BottomEnd => (Side::Bottom, Align::End),
            Left => (Side::Left, Align::Center),
            LeftStart => (Side::Left, Align::Start),
            LeftEnd => (Side::Left, Align::End),
            Right => (Side::Right, Align::Center),
            RightStart => (Side::Right, Align::Start),
            RightEnd => (Side::Right, Align::End),
        }
    }

    fn from_parts(side: Side, align: Align) -> Self {
        use PopoverPlacement::{
            Bottom, BottomEnd, BottomStart, Left, LeftEnd, LeftStart, Right, RightEnd, RightStart,
            Top, TopEnd, TopStart,
        };
        match (side, align) {
            (Side::Top, Align::Center) => Top,
            (Side::Top, Align::Start) => TopStart,
            (Side::Top, Align::End) => TopEnd,
            (Side::Bottom, Align::Center) => Bottom,
            (Side::Bottom, Align::Start) => BottomStart,
            (Side::Bottom, Align::End) => BottomEnd,
            (Side::Left, Align::Center) => Left,
            (Side::Left, Align::Start) => LeftStart,
            (Side::Left, Align::End) => LeftEnd,
            (Side::Right, Align::Center) => Right,
            (Side::Right, Align::Start) => RightStart,
            (Side::Right, Align::End) => RightEnd,
        }
    }

    /// Stable kebab-case string used as a `data-placement` attribute to
    /// drive the CSS entrance animation direction.
    pub fn data_attr(self) -> &'static str {
        use PopoverPlacement::{
            Bottom, BottomEnd, BottomStart, Left, LeftEnd, LeftStart, Right, RightEnd, RightStart,
            Top, TopEnd, TopStart,
        };
        match self {
            Top => "top",
            TopStart => "top-start",
            TopEnd => "top-end",
            Bottom => "bottom",
            BottomStart => "bottom-start",
            BottomEnd => "bottom-end",
            Left => "left",
            LeftStart => "left-start",
            LeftEnd => "left-end",
            Right => "right",
            RightStart => "right-start",
            RightEnd => "right-end",
        }
    }
}

#[component]
pub fn Popover(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] anchor: Signal<Option<web_sys::HtmlElement>>,
    /// Alternative anchor: a fixed point in viewport coordinates `(x, y)`.
    /// Used by Context Menu and similar pointer-triggered floating UI where
    /// there is no trigger element. When both `anchor` and `anchor_point` are
    /// Some, `anchor_point` wins.
    #[prop(into, default = Signal::stored(None))]
    anchor_point: Signal<Option<(f64, f64)>>,
    #[prop(optional)] placement: PopoverPlacement,
    #[prop(optional, default = 8.0)] offset: f64,
    #[prop(optional)] close_on_scroll: bool,
    #[prop(optional)] match_trigger_width: bool,
    children: ChildrenFn,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let mounted = RwSignal::new(false);
    let data_state = RwSignal::new(Option::<&'static str>::None);
    let resolved_placement = RwSignal::new(placement);
    let pos_top = RwSignal::new(0.0_f64);
    let pos_left = RwSignal::new(0.0_f64);
    let min_width: RwSignal<Option<f64>> = RwSignal::new(None);
    let positioned = RwSignal::new(false);

    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let previously_focused: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);
    let listeners: StoredValue<Vec<Box<dyn FnOnce()>>, LocalStorage> =
        StoredValue::new_local(Vec::new());
    let children_stored = StoredValue::new(children);

    let enter_ver = Arc::new(AtomicU32::new(0));
    let exit_ver = Arc::new(AtomicU32::new(0));

    let do_reposition = Callback::new(move |(): ()| {
        // Resolve anchor rect: `anchor_point` wins when present, else fall
        // back to element anchor.
        let anchor_rl = if let Some((x, y)) = anchor_point.get_untracked() {
            RectLike::from_point(x, y)
        } else if let Some(anchor_el) = anchor.get_untracked() {
            RectLike::from_dom(&anchor_el.get_bounding_client_rect())
        } else {
            return;
        };

        let Some(panel_div) = panel_ref.get_untracked() else {
            return;
        };
        let panel_el: &web_sys::HtmlElement = panel_div.unchecked_ref();
        let panel_rect = panel_el.get_bounding_client_rect();
        let Some(window) = web_sys::window() else {
            return;
        };
        let vw = window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let vh = window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let (resolved, top, left, mw) = calc_position(
            &anchor_rl,
            &panel_rect,
            (vw, vh),
            placement,
            offset,
            match_trigger_width,
        );
        resolved_placement.set(resolved);
        pos_top.set(top);
        pos_left.set(left);
        min_width.set(mw);
        positioned.set(true);
    });

    let detach_listeners = move || {
        listeners.update_value(|v| {
            for cleanup in v.drain(..) {
                cleanup();
            }
        });
    };

    let attach_listeners = move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };

        // pointerdown — close when click lands outside both panel and anchor
        {
            let closure = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
                move |ev: web_sys::PointerEvent| {
                    let Some(target) = ev.target() else {
                        return;
                    };
                    let Ok(target_node) = target.dyn_into::<web_sys::Node>() else {
                        return;
                    };

                    let in_panel = panel_ref.get_untracked().is_some_and(|p| {
                        let node: &web_sys::Node = p.unchecked_ref();
                        node.contains(Some(&target_node))
                    });
                    if in_panel {
                        return;
                    }
                    let in_anchor = anchor.get_untracked().is_some_and(|a| {
                        let node: &web_sys::Node = a.unchecked_ref();
                        node.contains(Some(&target_node))
                    });
                    if in_anchor {
                        return;
                    }
                    on_close.run(());
                },
            );
            let _ = document
                .add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref());
            let doc = document.clone();
            let cleanup: Box<dyn FnOnce()> = Box::new(move || {
                let _ = doc.remove_event_listener_with_callback(
                    "pointerdown",
                    closure.as_ref().unchecked_ref(),
                );
                drop(closure);
            });
            listeners.update_value(|v| v.push(cleanup));
        }

        // keydown — Escape closes
        {
            let closure = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        on_close.run(());
                    }
                },
            );
            let _ = document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            let doc = document;
            let cleanup: Box<dyn FnOnce()> = Box::new(move || {
                let _ = doc.remove_event_listener_with_callback(
                    "keydown",
                    closure.as_ref().unchecked_ref(),
                );
                drop(closure);
            });
            listeners.update_value(|v| v.push(cleanup));
        }

        // scroll — reposition or close
        {
            let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
                if close_on_scroll {
                    on_close.run(());
                } else {
                    do_reposition.run(());
                }
            });
            let _ =
                window.add_event_listener_with_callback("scroll", closure.as_ref().unchecked_ref());
            let win = window.clone();
            let cleanup: Box<dyn FnOnce()> = Box::new(move || {
                let _ = win.remove_event_listener_with_callback(
                    "scroll",
                    closure.as_ref().unchecked_ref(),
                );
                drop(closure);
            });
            listeners.update_value(|v| v.push(cleanup));
        }

        // resize — reposition
        {
            let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
                do_reposition.run(());
            });
            let _ =
                window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            let win = window;
            let cleanup: Box<dyn FnOnce()> = Box::new(move || {
                let _ = win.remove_event_listener_with_callback(
                    "resize",
                    closure.as_ref().unchecked_ref(),
                );
                drop(closure);
            });
            listeners.update_value(|v| v.push(cleanup));
        }
    };

    let ev_enter = enter_ver;
    let ev_exit = exit_ver;
    Effect::new(move |prev: Option<bool>| {
        let now = open.get();
        let was = prev.unwrap_or(false);

        if now && !was {
            ev_exit.fetch_add(1, Ordering::Relaxed);
            let ticket = ev_enter.fetch_add(1, Ordering::Relaxed) + 1;

            let active_el = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element())
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
            previously_focused.set_value(active_el);

            positioned.set(false);
            data_state.set(None);
            mounted.set(true);

            let ev = Arc::clone(&ev_enter);
            set_timeout(
                move || {
                    if ev.load(Ordering::Relaxed) != ticket {
                        return;
                    }
                    do_reposition.run(());
                    attach_listeners();
                    focus_first_or_panel(panel_ref);
                },
                Duration::ZERO,
            );
        } else if !now && was {
            ev_enter.fetch_add(1, Ordering::Relaxed);
            let ticket = ev_exit.fetch_add(1, Ordering::Relaxed) + 1;

            data_state.set(Some("closing"));
            detach_listeners();

            let ev = Arc::clone(&ev_exit);
            set_timeout(
                move || {
                    if ev.load(Ordering::Relaxed) != ticket {
                        return;
                    }
                    mounted.set(false);
                    data_state.set(None);
                    positioned.set(false);

                    if let Some(el) = previously_focused.get_value() {
                        let _ = el.focus();
                        previously_focused.set_value(None);
                    }
                },
                Duration::from_millis(100),
            );
        }
        now
    });

    on_cleanup(move || {
        detach_listeners();
    });

    let panel_class = StoredValue::new(["popover-panel", class].join(" "));

    view! {
        <Show when=move || mounted.get()>
            <leptos::portal::Portal>
                <div
                    node_ref=panel_ref
                    class=panel_class.get_value()
                    role="dialog"
                    aria-modal="false"
                    tabindex="-1"
                    data-state=move || data_state.get()
                    data-placement=move || resolved_placement.get().data_attr()
                    data-positioned=move || { if positioned.get() { "true" } else { "false" } }
                    style=move || {
                        let mw = min_width
                            .get()
                            .map(|w| format!("min-width: {w}px;"))
                            .unwrap_or_default();
                        format!("top: {}px; left: {}px; {}", pos_top.get(), pos_left.get(), mw)
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Tab" {
                            handle_tab(&ev, panel_ref, on_close);
                        }
                    }
                >
                    {move || children_stored.with_value(|c| c())}
                </div>
            </leptos::portal::Portal>
        </Show>
    }
}

/// Focus the first focusable descendant of the panel, or the panel itself
/// (which has `tabindex="-1"`) if none exists.
fn focus_first_or_panel(panel_ref: NodeRef<leptos::html::Div>) {
    let Some(root) = panel_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::HtmlElement = root.unchecked_ref();
    if let Ok(list) = el.query_selector_all(FOCUSABLE_SELECTOR) {
        if let Some(node) = list.item(0) {
            if let Ok(focusable) = node.dyn_into::<web_sys::HtmlElement>() {
                let _ = focusable.focus();
                return;
            }
        }
    }
    let _ = el.focus();
}

/// When focus is about to leave the panel via Tab / Shift+Tab, call `on_close`
/// and let the focus continue naturally to the next element in the document.
fn handle_tab(
    ev: &web_sys::KeyboardEvent,
    panel_ref: NodeRef<leptos::html::Div>,
    on_close: Callback<()>,
) {
    let Some(root) = panel_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::HtmlElement = root.unchecked_ref();
    let Ok(list) = el.query_selector_all(FOCUSABLE_SELECTOR) else {
        return;
    };
    let len = list.length();
    if len == 0 {
        // Empty panel: any Tab exits
        on_close.run(());
        return;
    }

    let first = list
        .item(0)
        .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok());
    let last = list
        .item(len - 1)
        .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok());
    let active = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());

    let same = |a: &web_sys::Element, b: &web_sys::HtmlElement| -> bool {
        js_sys::Object::is(a.as_ref(), b.as_ref())
    };

    let is_first = matches!((active.as_ref(), first.as_ref()), (Some(a), Some(f)) if same(a, f));
    let is_last = matches!((active.as_ref(), last.as_ref()), (Some(a), Some(l)) if same(a, l));

    if (ev.shift_key() && is_first) || (!ev.shift_key() && is_last) {
        on_close.run(());
    }
    // Otherwise, Tab moves between items inside the panel — default behavior.
}

/// Positioning-agnostic rectangle. Used so both real DOM rects (from
/// `get_bounding_client_rect()`) and synthetic rects from pointer coordinates
/// (Context Menu) flow through the same positioning pipeline.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RectLike {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
    pub width: f64,
    pub height: f64,
}

impl RectLike {
    fn from_dom(rect: &web_sys::DomRect) -> Self {
        Self {
            top: rect.top(),
            bottom: rect.bottom(),
            left: rect.left(),
            right: rect.right(),
            width: rect.width(),
            height: rect.height(),
        }
    }

    fn from_point(x: f64, y: f64) -> Self {
        Self {
            top: y,
            bottom: y,
            left: x,
            right: x,
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Pure position calculation: primary-axis auto-flip + cross-axis clamp.
fn calc_position(
    anchor_rect: &RectLike,
    panel_rect: &web_sys::DomRect,
    viewport: (f64, f64),
    placement: PopoverPlacement,
    offset: f64,
    match_trigger_width: bool,
) -> (PopoverPlacement, f64, f64, Option<f64>) {
    let (vw, vh) = viewport;
    let ar_top = anchor_rect.top;
    let ar_bottom = anchor_rect.bottom;
    let ar_left = anchor_rect.left;
    let ar_right = anchor_rect.right;
    let ar_width = anchor_rect.width;
    let ar_height = anchor_rect.height;
    let pw = panel_rect.width();
    let ph = panel_rect.height();

    let (mut side, align) = placement.parts();

    // Auto-flip primary axis
    match side {
        Side::Bottom if ar_bottom + ph + offset > vh && ar_top - ph - offset >= 0.0 => {
            side = Side::Top;
        }
        Side::Top if ar_top - ph - offset < 0.0 && ar_bottom + ph + offset <= vh => {
            side = Side::Bottom;
        }
        Side::Right if ar_right + pw + offset > vw && ar_left - pw - offset >= 0.0 => {
            side = Side::Left;
        }
        Side::Left if ar_left - pw - offset < 0.0 && ar_right + pw + offset <= vw => {
            side = Side::Right;
        }
        _ => {}
    }

    let (mut top, mut left) = match side {
        Side::Bottom => (ar_bottom + offset, 0.0),
        Side::Top => (ar_top - ph - offset, 0.0),
        Side::Right => (0.0, ar_right + offset),
        Side::Left => (0.0, ar_left - pw - offset),
    };

    match side {
        Side::Top | Side::Bottom => {
            left = match align {
                Align::Start => ar_left,
                Align::Center => ar_left + (ar_width - pw) / 2.0,
                Align::End => ar_right - pw,
            };
        }
        Side::Left | Side::Right => {
            top = match align {
                Align::Start => ar_top,
                Align::Center => ar_top + (ar_height - ph) / 2.0,
                Align::End => ar_bottom - ph,
            };
        }
    }

    // Auto-shift — clamp within viewport with 8px margin
    let max_left = (vw - pw - 8.0).max(8.0);
    let max_top = (vh - ph - 8.0).max(8.0);
    left = left.max(8.0).min(max_left);
    top = top.max(8.0).min(max_top);

    let resolved = PopoverPlacement::from_parts(side, align);
    let mw = if match_trigger_width {
        Some(ar_width)
    } else {
        None
    };
    (resolved, top, left, mw)
}
