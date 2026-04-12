use crate::pages::{
    not_found::NotFoundPage, page::Page as EntryPage, playground::PlayGroundRoutes, v::VRoutes,
};
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component(transparent)]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/") view=EntryPage />
                <VRoutes />

                // ....
                <PlayGroundRoutes />
            </Routes>
        </Router>
    }
}
