use crate::primitives::tokens::Size;
use leptos::prelude::*;

#[component]
pub fn Spinner(
    #[prop(optional)] size: Size,
    #[prop(optional, default = "Loading")] label: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let cls = ["spinner", size.spinner_class(), class].join(" ");

    view! { <span class=cls role="status" aria-label=label></span> }
}
