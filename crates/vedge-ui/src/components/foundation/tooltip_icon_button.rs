use crate::components::foundation::icon_button::IconButton;
use crate::components::feedback::tooltip::Tooltip;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Placement, Shape, Size, Variant};
use leptos::prelude::*;

/// Default composition of IconButton + Tooltip for production icon-only actions.
///
/// The single `label` prop drives both the tooltip text and the IconButton's
/// `aria-label`, guaranteeing the two never drift. When `disabled=true`, the
/// Tooltip short-circuits and does not mount — a disabled action's label is
/// not actionable, so the tooltip would be noise.
#[component]
pub fn TooltipIconButton(
    children: Children,
    #[prop(into)] label: TextProp,
    #[prop(optional, default = Variant::Ghost)] variant: Variant,
    #[prop(optional)] size: Size,
    #[prop(optional)] shape: Shape,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(into, default = None)] on_click: Option<Callback<()>>,
    #[prop(optional)] tooltip_placement: Placement,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"TooltipIconButton: `label` is required — it provides both the tooltip text and the button's aria-label.".into(),
        );
    }

    let tooltip_content = Signal::derive(move || label.get());

    let handle_click = move |_: web_sys::MouseEvent| {
        if let Some(cb) = on_click {
            cb.run(());
        }
    };

    let wrapper_cls = ["tooltip-icon-btn", class].join(" ");

    view! {
        <span class=wrapper_cls>
            <Tooltip
                content=tooltip_content
                placement=tooltip_placement
                disabled=disabled
            >
                <IconButton
                    aria_label=label
                    variant=variant
                    size=size
                    shape=shape
                    disabled=disabled
                    loading=loading
                    on:click=handle_click
                >
                    {children()}
                </IconButton>
            </Tooltip>
        </span>
    }
}
