//! `HelpPopover` — a small `?` affordance that opens a short help sentence on
//! click. A thin composition of the design-system [`Popover`] (controlled,
//! trigger-less) and [`IconButton`]; it owns its open state, so a caller only
//! supplies the help `body` and an aria `label` (both `Signal<String>` so they
//! relocalise and are read under a reactive owner).
//!
//! Lives in `vedge-app` rather than `vedge-ui` deliberately: it introduces no
//! new visual primitive, token, or CSS — it wires two existing primitives — and
//! its only consumers are app features (the contextual `?`s in Settings and on
//! the launch screen). Promote it to the design system (with the mandated
//! playground) if a second, DS-level inline-help need ever appears.
//!
//! 🔴 Scroll caveat: `Popover` binds its reposition/close `scroll` listener to
//! `window` in the bubble phase, so an `?` opened inside an inner-scroll panel
//! (the Pattern-B Settings tabs) does not reposition on that panel's scroll — it
//! floats until the next click (which closes it). Keep bodies short (one glance);
//! a capture-phase / `scroll_root` fix on `Popover` is the tracked follow-up.

use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::foundation::icon_button::IconButton;
use vedge_ui::components::popover::{Popover, PopoverPlacement};
use vedge_ui::primitives::tokens::{Shape, Size, Variant};

#[component]
pub fn HelpPopover(
    /// The help sentence(s). `Signal<String>` so it relocalises and is read under
    /// a reactive owner — pass `Signal::derive(move || t_string!(…).to_owned())`.
    #[prop(into)]
    body: Signal<String>,
    /// aria-label for the icon-only `?` trigger.
    #[prop(into)]
    label: Signal<String>,
    #[prop(optional, default = PopoverPlacement::BottomStart)] placement: PopoverPlacement,
    /// Optional e2e hook on the wrapping span (stable across open/close).
    #[prop(optional, default = "")]
    testid: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    // IconButton does not forward `node_ref`, so anchor the Popover to a tight
    // `inline-flex` span wrapping the button (it stays coincident with the `?`).
    let trigger_ref = NodeRef::<leptos::html::Span>::new();
    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    view! {
        <span
            node_ref=trigger_ref
            class="inline-flex"
            data-testid=(!testid.is_empty()).then_some(testid)
        >
            <IconButton
                aria_label=label
                variant=Variant::Ghost
                size=Size::Sm
                shape=Shape::Pill
                // A second `?` click toggles closed: the anchor span counts as
                // "inside" for the Popover's outside-pointerdown, so the toggle
                // (not the dismiss) wins.
                on_click=Callback::new(move |_: ()| open.update(|v| *v = !*v))
            >
                <span aria-hidden="true">
                    <Icon icon=i::FaCircleQuestionSolid />
                </span>
            </IconButton>
            <Popover
                open=open
                on_close=Callback::new(move |_: ()| open.set(false))
                anchor=anchor
                placement=placement
            >
                <div
                    class="p-3 max-w-xs text-xs leading-relaxed text-text-secondary"
                    data-testid="help-popover-body"
                >
                    {move || body.get()}
                </div>
            </Popover>
        </span>
    }
}
