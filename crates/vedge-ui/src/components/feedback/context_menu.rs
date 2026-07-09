use crate::components::feedback::dropdown_menu::{MenuPanelBody, MenuSection, MenuState};
use crate::components::feedback::popover::{Popover, PopoverPlacement};
use crate::primitives::text_prop::TextProp;
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

/// Right-click triggered floating action list. Shares item rendering and
/// keyboard contract with DropdownMenu. Anchors to the pointer position via
/// Popover's `anchor_point` prop. Focus restores to the element that was
/// focused before the right-click (not a trigger, since there isn't one).
///
/// When `disabled=true`, the browser's native context menu is shown — we do
/// NOT call `event.preventDefault()`. Calling `preventDefault` on a disabled
/// component would leave users with no context menu at all.
#[component]
pub fn ContextMenu(
    children: Children,
    items: Vec<MenuSection>,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = None)] on_open: Option<Callback<()>>,
    #[prop(into, default = None)] on_close: Option<Callback<()>>,
    #[prop(into, default = TextProp::from("Context menu"))] aria_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let click_point: RwSignal<Option<(f64, f64)>> = RwSignal::new(None);
    let previous_focus: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);

    let state = MenuState::new(items);

    let do_close_and_restore = Callback::new(move |(): ()| {
        set_open.set(false);
        if let Some(cb) = on_close {
            cb.run(());
        }
        if let Some(el) = previous_focus.get_value() {
            // Only refocus if the element is still connected to the document.
            let doc = web_sys::window().and_then(|w| w.document());
            if let Some(doc) = doc {
                let doc_node: &web_sys::Node = doc.unchecked_ref();
                let el_node: &web_sys::Node = el.unchecked_ref();
                if doc_node.contains(Some(el_node)) {
                    let _ = el.focus();
                }
            }
            previous_focus.set_value(None);
        }
    });

    let handle_contextmenu = move |ev: web_sys::MouseEvent| {
        if disabled {
            // Leave the native context menu alone.
            return;
        }
        ev.prevent_default();
        let x = f64::from(ev.client_x());
        let y = f64::from(ev.client_y());
        click_point.set(Some((x, y)));

        // Capture focused element for restoration on close.
        let active = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
        previous_focus.set_value(active);

        set_open.set(true);
        if let Some(cb) = on_open {
            cb.run(());
        }

        // After Popover mounts, focus the first focusable item.
        let focused_idx = state.focused_idx;
        let panel_ref = state.panel_ref;
        let focusable_count = state.focusable_count;
        if focusable_count > 0 {
            focused_idx.set(Some(0));
            set_timeout(
                move || {
                    if let Some(panel) = panel_ref.get_untracked() {
                        let el: &web_sys::Element = panel.unchecked_ref();
                        if let Ok(Some(node)) = el.query_selector("[data-menu-idx=\"0\"]") {
                            if let Ok(btn) = node.dyn_into::<web_sys::HtmlElement>() {
                                let _ = btn.focus();
                            }
                        }
                    }
                },
                Duration::from_millis(20),
            );
        }
    };

    let wrapper_cls = ["context-menu-target", class].join(" ");

    view! {
        <div class=wrapper_cls on:contextmenu=handle_contextmenu>
            {children()}
        </div>

        <Popover
            open=Signal::derive(move || open.get())
            on_close=do_close_and_restore
            // Element anchor unused — Popover will fall back to anchor_point.
            anchor=Signal::stored(None)
            anchor_point=Signal::derive(move || click_point.get())
            placement=PopoverPlacement::BottomStart
        >
            <MenuPanelBody
                items=state.items
                focused_idx=state.focused_idx
                focusable_count=state.focusable_count
                focusable_ids=state.focusable_ids
                panel_ref=state.panel_ref
                typeahead_buf=state.typeahead_buf
                typeahead_ver=state.typeahead_ver
                close_cb=do_close_and_restore
                aria_label=aria_label
            />
        </Popover>
    }
}
