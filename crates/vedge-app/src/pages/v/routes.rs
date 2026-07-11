use super::layout::VLayout;
use super::manage::{
    audit::AuditPage, generator::GeneratorPage, health::HealthPage, settings::SettingsPage,
    vault::VaultPage,
};
use super::page::VPage;
use leptos::prelude::*;
use leptos_router::{
    MatchNestedRoutes,
    components::{ParentRoute, Route},
    path,
};

#[component(transparent)]
pub fn VRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/v") view=VLayout>
            <Route path=path!("/") view=VPage />
            <Route path=path!("/vault") view=VaultPage />
            <Route path=path!("/audit") view=AuditPage />
            <Route path=path!("/health") view=HealthPage />
            <Route path=path!("/generator") view=GeneratorPage />
            <Route path=path!("/settings") view=SettingsPage />
        </ParentRoute>
    }
    .into_inner()
}
