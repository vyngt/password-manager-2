use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn VPage() -> impl IntoView {
    let nav = use_navigate();
    nav("/v/vault", Default::default());

    view! { <div class="h-full"></div> }
}
