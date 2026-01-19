use leptos::prelude::*;

#[component]
pub fn TableRow(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! { <tr role="row" class=class>{children()}</tr> }
}
