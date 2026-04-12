use leptos::prelude::*;

#[component]
pub fn TableBody(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! { <tbody class=class>{children()}</tbody> }
}
