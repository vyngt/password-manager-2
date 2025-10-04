use crate::pages::page::Page as EntryPage;
use crate::pages::playground::PlayGroundRoutes;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component(transparent)]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                <PlayGroundRoutes />
                <Route path=path!("/") view=EntryPage />
            </Routes>
        </Router>
    }
}
