use crate::primitives::text_prop::TextProp;
use leptos::prelude::*;
use leptos_icons::Icon;

/// A shared "nothing here" placeholder: a decorative icon, a title, an optional
/// description line, and an optional call-to-action slot (usually a `Button`).
///
/// Use it to make an *empty result* look distinct from a *loading* state (which
/// should render a [`Spinner`](super::spinner::Spinner)) — the two are otherwise
/// easy to confuse when both are bare gray text.
///
/// The icon is decorative (`aria-hidden`); the accessible content is the title
/// (and description, when present).
#[component]
pub fn EmptyState(
    /// Decorative glyph shown above the title (e.g. `icondata::FaInboxSolid`).
    #[prop(optional)]
    icon: Option<icondata_core::Icon>,
    /// The headline — what's missing. Reactive so it relocalizes.
    #[prop(into, default = TextProp::default())]
    title: TextProp,
    /// Optional secondary line under the title. Omitted when empty.
    #[prop(into, default = TextProp::default())]
    description: TextProp,
    /// Optional call-to-action, rendered below the text (pass a `<Button>`).
    #[prop(optional)]
    children: Option<Children>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let root_cls = ["empty-state", class].join(" ");

    view! {
        <div class=root_cls>
            {icon
                .map(|ic| {
                    view! {
                        <span class="empty-state__icon" aria-hidden="true">
                            <Icon icon=ic />
                        </span>
                    }
                })}
            <div class="empty-state__title">{move || title.get()}</div>
            {move || {
                let d = description.get();
                (!d.is_empty())
                    .then(|| view! { <div class="empty-state__description">{d}</div> })
            }}
            {children.map(|c| view! { <div class="empty-state__action">{c()}</div> })}
        </div>
    }
}
