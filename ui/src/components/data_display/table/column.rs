use leptos::prelude::*;

#[component]
pub fn TableColumn(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! { <th class=class>{children()}</th> }
}
