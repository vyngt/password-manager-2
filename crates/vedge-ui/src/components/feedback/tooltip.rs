use crate::primitives::tokens::Placement;
use crate::utils::id::id_with_prefix;
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[component]
pub fn Tooltip(
    children: Children,
    #[prop(into)] content: Signal<String>,
    #[prop(optional)] placement: Placement,
    #[prop(optional, default = 500)] delay: u32,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] arrow: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    if disabled {
        return children().into_any();
    }

    // content is Signal<String> — reactive, works with i18n
    let trigger_ref: NodeRef<leptos::html::Span> = NodeRef::new();
    let tooltip_id: String = id_with_prefix("tooltip");

    let mounted = RwSignal::new(false);
    let data_state = RwSignal::new(Option::<&'static str>::None);
    let style = RwSignal::new(String::new());

    // Arc<AtomicU32> — safe to access from set_timeout (outside reactive owner)
    let show_ver = Arc::new(AtomicU32::new(0));
    let hide_ver = Arc::new(AtomicU32::new(0));

    // Calculate tooltip position from trigger bounding rect
    let update_position = move || {
        if let Some(el) = trigger_ref.get() {
            let rect = el.get_bounding_client_rect();
            let (x, y, tx, ty) = match placement {
                Placement::Top => (
                    rect.left() + rect.width() / 2.0,
                    rect.top() - 8.0,
                    "-50%",
                    "-100%",
                ),
                Placement::Bottom => (
                    rect.left() + rect.width() / 2.0,
                    rect.bottom() + 8.0,
                    "-50%",
                    "0",
                ),
                Placement::Left => (
                    rect.left() - 8.0,
                    rect.top() + rect.height() / 2.0,
                    "-100%",
                    "-50%",
                ),
                Placement::Right => (
                    rect.right() + 8.0,
                    rect.top() + rect.height() / 2.0,
                    "0",
                    "-50%",
                ),
            };
            style.set(format!("left:{x}px;top:{y}px;translate:{tx} {ty};"));
        }
    };

    // Show: position → mount → animate on next tick
    let sv = Arc::clone(&show_ver);
    let hv = Arc::clone(&hide_ver);
    let do_show = move || {
        hv.fetch_add(1, Ordering::Relaxed);
        update_position();
        data_state.set(None);
        mounted.set(true);
        let ver = sv.load(Ordering::Relaxed);
        let sv2 = Arc::clone(&sv);
        set_timeout(
            move || {
                if sv2.load(Ordering::Relaxed) == ver {
                    data_state.set(Some("open"));
                }
            },
            Duration::ZERO,
        );
    };

    // Hide: exit animation → unmount after 150ms
    let sv = Arc::clone(&show_ver);
    let hv = Arc::clone(&hide_ver);
    let do_hide = move || {
        sv.fetch_add(1, Ordering::Relaxed);
        data_state.set(Some("closed"));
        let ver = hv.load(Ordering::Relaxed);
        let hv2 = Arc::clone(&hv);
        set_timeout(
            move || {
                if hv2.load(Ordering::Relaxed) == ver {
                    mounted.set(false);
                    data_state.set(None);
                }
            },
            Duration::from_millis(150),
        );
    };

    // Hover: show after configurable delay
    let sv = Arc::clone(&show_ver);
    let hv = Arc::clone(&hide_ver);
    let do_show_c = do_show.clone();
    let on_mouseenter = move |_| {
        let ver = sv.fetch_add(1, Ordering::Relaxed) + 1;
        hv.fetch_add(1, Ordering::Relaxed);
        if delay == 0 {
            do_show_c();
        } else {
            let sv2 = Arc::clone(&sv);
            let ds = do_show_c.clone();
            set_timeout(
                move || {
                    if sv2.load(Ordering::Relaxed) == ver {
                        ds();
                    }
                },
                Duration::from_millis(u64::from(delay)),
            );
        }
    };

    // Mouse leave: 100ms grace period before hiding
    let sv = Arc::clone(&show_ver);
    let hv = Arc::clone(&hide_ver);
    let do_hide_c = do_hide.clone();
    let on_mouseleave = move |_| {
        sv.fetch_add(1, Ordering::Relaxed);
        let ver = hv.fetch_add(1, Ordering::Relaxed) + 1;
        let hv2 = Arc::clone(&hv);
        let dh = do_hide_c.clone();
        set_timeout(
            move || {
                if hv2.load(Ordering::Relaxed) == ver {
                    dh();
                }
            },
            Duration::from_millis(100),
        );
    };

    // Focus: immediate show (0ms delay for keyboard)
    let sv = Arc::clone(&show_ver);
    let hv = Arc::clone(&hide_ver);
    let do_show_c2 = do_show.clone();
    let on_focusin = move |_| {
        sv.fetch_add(1, Ordering::Relaxed);
        hv.fetch_add(1, Ordering::Relaxed);
        do_show_c2();
    };

    // Blur: immediate hide with exit animation
    let sv = show_ver;
    let hv = hide_ver;
    let on_focusout = move |_| {
        sv.fetch_add(1, Ordering::Relaxed);
        hv.fetch_add(1, Ordering::Relaxed);
        do_hide();
    };

    let panel_cls = format!("tooltip-panel {class}");
    let placement_str = placement.as_str();
    let id_for_panel = tooltip_id.clone();
    let id_for_aria = tooltip_id;

    view! {
        <span
            node_ref=trigger_ref
            style:display="inline-flex"
            on:mouseenter=on_mouseenter
            on:mouseleave=on_mouseleave
            on:focusin=on_focusin
            on:focusout=on_focusout
            aria-describedby=move || {
                if mounted.get() { Some(id_for_aria.clone()) } else { None }
            }
        >
            {children()}
        </span>

        <Show when=move || mounted.get()>
            <div
                id=id_for_panel.clone()
                class=panel_cls.clone()
                style=move || style.get()
                role="tooltip"
                data-state=move || data_state.get()
                data-placement=placement_str
            >
                {move || content.get()}
                {arrow.then(|| view! { <div class="tooltip-arrow"></div> })}
            </div>
        </Show>
    }
    .into_any()
}
