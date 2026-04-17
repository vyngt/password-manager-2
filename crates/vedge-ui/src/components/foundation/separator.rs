use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Orientation;
use leptos::prelude::*;

#[component]
pub fn Separator(
    #[prop(optional)] orientation: Orientation,
    #[prop(into, default = TextProp::default())] label: TextProp,
    #[prop(optional)] strong: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let strong_cls = if strong { "separator--strong" } else { "" };

    if !label.get_untracked().is_empty() {
        let cls = ["separator", "separator--labeled", strong_cls, class].join(" ");

        view! {
            <div class=cls role="separator">
                <span class="separator__line"></span>
                <span class="separator__label">{move || label.get()}</span>
                <span class="separator__line"></span>
            </div>
        }
        .into_any()
    } else if orientation == Orientation::Vertical {
        let cls = ["separator", "separator--vertical", strong_cls, class].join(" ");

        view! { <div class=cls role="separator" aria-orientation="vertical"></div> }
        .into_any()
    } else {
        let cls = [
            "separator",
            "separator--horizontal",
            strong_cls,
            class,
        ]
        .join(" ");

        view! { <hr class=cls /> }
        .into_any()
    }
}
