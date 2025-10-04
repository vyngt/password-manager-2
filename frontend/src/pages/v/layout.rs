use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn VLayout() -> impl IntoView {
    view! { <Outlet /> }
}
