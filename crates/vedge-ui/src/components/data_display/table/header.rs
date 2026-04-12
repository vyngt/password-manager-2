use leptos::prelude::*;

use crate::utils;

#[component]
pub fn TableHeader(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = "bg-gray-100/20";
    let cls = utils::string::merge_classnames(vec![base_cls, class]);
    view! { <thead class=cls>{children()}</thead> }
}
