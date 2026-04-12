use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Size;
use leptos::prelude::*;

#[component]
pub fn Spinner(
    #[prop(optional)] size: Size,
    #[prop(into, default = TextProp::default())] label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let cls = ["spinner", size.spinner_class(), class].join(" ");

    view! { <span class=cls role="status" aria-label=move || { let v = label.get(); if v.is_empty() { None } else { Some(v) } }></span> }
}
