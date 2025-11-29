use leptos::prelude::*;

#[component]
pub fn TableHeader(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! { <thead class=class>{children()}</thead> }
}
