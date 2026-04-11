use crate::primitives::tokens::{BadgeAppearance, BadgeShape, BadgeSize, BadgeVariant};
use leptos::prelude::*;

#[component]
pub fn Badge(
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] variant: BadgeVariant,
    #[prop(optional)] size: BadgeSize,
    #[prop(optional)] shape: BadgeShape,
    #[prop(optional)] appearance: BadgeAppearance,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let is_dot = shape == BadgeShape::Dot;

    let cls = [
        "badge",
        variant.badge_class(),
        size.badge_class(),
        shape.badge_class(),
        if is_dot { "badge--solid" } else { appearance.badge_class() },
        class,
    ]
    .join(" ");

    view! {
        <span class=cls aria-hidden={if is_dot { Some("true") } else { None }}>
            {if is_dot { None } else { children.map(|c| c()) }}
        </span>
    }
}
