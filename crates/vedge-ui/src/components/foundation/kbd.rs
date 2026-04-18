use leptos::prelude::*;

#[component]
pub fn Kbd(
    children: Children,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let cls = ["kbd", class].join(" ");
    view! { <kbd class=cls>{children()}</kbd> }
}
