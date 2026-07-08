use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::DialogSize;
use leptos::prelude::*;
use std::time::Duration;
use wasm_bindgen::JsCast;

const FOCUSABLE_SELECTOR: &str = "button:not([disabled]), [href], input:not([disabled]), \
    select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

/// Context shared from `Dialog` to its subtree.
///
/// `Copy` because every field is `Copy`: `Callback<()>` wraps a `StoredValue`,
/// `WriteSignal` is `Copy`, and `TextProp` wraps a `Signal<String>`.
#[derive(Clone, Copy)]
pub struct DialogContext {
    pub on_close: Callback<()>,
    pub closeable: bool,
    pub close_label: TextProp,
    pub set_title_id: WriteSignal<Option<String>>,
    pub set_body_id: WriteSignal<Option<String>>,
}

#[component]
pub fn Dialog(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(optional)] size: DialogSize,
    #[prop(optional, default = true)] closeable: bool,
    #[prop(into, default = TextProp::from("Close"))] close_label: TextProp,
    children: ChildrenFn,
) -> impl IntoView {
    // Animation state only (`None` | `"open"`); visibility is driven directly by
    // `open` (see the `<Show>` below), NOT by an internal `mounted` signal — that
    // internal signal desynced from the on-screen dialog when the component
    // re-rendered, leaving dialogs that couldn't be closed.
    let data_state = RwSignal::new(Option::<&'static str>::None);

    let (title_id, set_title_id) = signal(Option::<String>::None);
    let (body_id, set_body_id) = signal(Option::<String>::None);

    let previously_focused: StoredValue<Option<web_sys::HtmlElement>> = StoredValue::new(None);
    let dialog_ref = NodeRef::<leptos::html::Div>::new();
    let children_stored = StoredValue::new(children);

    // Defer the close one macrotask so the `<Portal>` teardown never runs
    // synchronously from within a portaled event handler. Closing from a
    // deeply-nested (type-erased) element inside the Portal — the header ✕ —
    // otherwise flips `open` but fails to unmount the Portal DOM (the
    // concrete-view backdrop happens to work). This mirrors the deferred unmount
    // every other Portal overlay uses (Popover / ColorPicker). Keeping `<Show>`
    // gated on the external `open` (no internal `mounted`) avoids the desync a
    // prior internal-visibility-signal implementation hit.
    let request_close = Callback::new(move |()| {
        set_timeout(move || on_close.run(()), Duration::ZERO);
    });

    // Provide context for sub-components. The ✕ closes via `request_close`
    // (deferred) so its in-portal click doesn't tear the Portal down synchronously.
    provide_context(DialogContext {
        on_close: request_close,
        closeable,
        close_label,
        set_title_id,
        set_body_id,
    });

    // Open/close driver — watches `open` and runs enter/exit lifecycles.
    // Side effects only — visibility is the `<Show when=open>` below. On open:
    // capture focus, lock scroll, and flip `data_state` to "open" next tick (after
    // mount) so the enter keyframe runs + focus lands. On close: reset + unlock +
    // restore focus. No internal `mounted`/exit-animation state to desync.
    Effect::new(move |_| {
        if open.get() {
            let active_el = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element())
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
            previously_focused.set_value(active_el);

            data_state.set(None);
            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                let _ = body.class_list().add_1("dialog-scroll-lock");
            }

            set_timeout(
                move || {
                    data_state.set(Some("open"));
                    focus_first_focusable(dialog_ref);
                },
                Duration::ZERO,
            );
        } else {
            data_state.set(None);
            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                let _ = body.class_list().remove_1("dialog-scroll-lock");
            }
            if let Some(el) = previously_focused.get_value() {
                let _ = el.focus();
                previously_focused.set_value(None);
            }
        }
    });

    // Ensure scroll-lock class is removed if the Dialog is unmounted
    // mid-animation (e.g. parent route change).
    on_cleanup(move || {
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body.class_list().remove_1("dialog-scroll-lock");
        }
    });

    let size_cls = size.dialog_class();

    view! {
        <Show when=move || open.get()>
            <leptos::portal::Portal>
                <div
                    class="dialog-scrim"
                    data-state=move || data_state.get()
                    on:click=move |ev: web_sys::MouseEvent| {
                        if !closeable {
                            return;
                        }
                        if let (Some(t), Some(c)) = (ev.target(), ev.current_target()) {
                            if js_sys::Object::is(t.as_ref(), c.as_ref()) {
                                request_close.run(());
                            }
                        }
                    }
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        match ev.key().as_str() {
                            "Escape" if closeable => {
                                ev.prevent_default();
                                request_close.run(());
                            }
                            "Tab" => trap_tab(&ev, dialog_ref),
                            _ => {}
                        }
                    }
                >
                    <div
                        node_ref=dialog_ref
                        class=format!("dialog {size_cls}")
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby=move || title_id.get()
                        aria-describedby=move || body_id.get()
                        tabindex="-1"
                    >
                        {move || children_stored.with_value(|c| c())}
                    </div>
                </div>
            </leptos::portal::Portal>
        </Show>
    }
}

/// Focus the first focusable element in the dialog, or the dialog container
/// itself (which has `tabindex="-1"`) if none exists.
fn focus_first_focusable(dialog_ref: NodeRef<leptos::html::Div>) {
    let Some(root) = dialog_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::HtmlElement = &root;

    if let Ok(list) = el.query_selector_all(FOCUSABLE_SELECTOR) {
        if let Some(node) = list.item(0) {
            if let Ok(focusable) = node.dyn_into::<web_sys::HtmlElement>() {
                let _ = focusable.focus();
                return;
            }
        }
    }

    // Fallback: focus the dialog container itself.
    let _ = el.focus();
}

/// Trap Tab / Shift+Tab within the dialog. Wraps at first/last focusable.
fn trap_tab(ev: &web_sys::KeyboardEvent, dialog_ref: NodeRef<leptos::html::Div>) {
    let Some(root) = dialog_ref.get_untracked() else {
        return;
    };
    let el: &web_sys::HtmlElement = &root;

    let list = match el.query_selector_all(FOCUSABLE_SELECTOR) {
        Ok(l) => l,
        Err(_) => return,
    };

    let len = list.length();
    if len == 0 {
        ev.prevent_default();
        let _ = el.focus();
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

    let same_identity = |a: &web_sys::Element, b: &web_sys::HtmlElement| -> bool {
        js_sys::Object::is(a.as_ref(), b.as_ref())
    };

    let is_first =
        matches!((active.as_ref(), first.as_ref()), (Some(a), Some(f)) if same_identity(a, f));
    let is_last =
        matches!((active.as_ref(), last.as_ref()), (Some(a), Some(l)) if same_identity(a, l));
    let is_root = active
        .as_ref()
        .map(|a| js_sys::Object::is(a.as_ref(), el.as_ref()))
        .unwrap_or(false);

    if ev.shift_key() {
        if is_first || is_root {
            ev.prevent_default();
            if let Some(el) = last {
                let _ = el.focus();
            }
        }
    } else if is_last {
        ev.prevent_default();
        if let Some(el) = first {
            let _ = el.focus();
        }
    }
}
