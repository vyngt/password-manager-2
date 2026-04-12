use leptos::prelude::*;

#[component]
pub fn TableCell(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! { <td class=class>{children()}</td> }
}
