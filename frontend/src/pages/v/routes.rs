use super::layout::VLayout;
use super::manage::page::ManagePage;
use super::page::VPage;
use leptos::prelude::*;
use leptos_router::{MatchNestedRoutes, components::*, path};

#[component(transparent)]
pub fn VRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/v") view=VLayout>
            <Route path=path!("/") view=VPage />
            <Route path=path!("/manage") view=ManagePage />
        </ParentRoute>
    }
    .into_inner()
}
