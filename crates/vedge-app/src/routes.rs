use crate::features::vault::vault_setup::VaultSetup;
#[cfg(debug_assertions)]
use crate::pages::playground::PlayGroundRoutes;
use crate::pages::{not_found::NotFoundPage, page::Page as EntryPage, v::VRoutes};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

// `/playground` is a dev-only component gallery, compiled out of release builds
// (PG.2a): unaudited dev surface in a password manager, dead weight in the bundle.
// A `#[cfg]` on a single `<Routes>` child breaks the router's child-tuple type
// inference, so the router is split into two whole functions instead — the release
// variant simply omits `<PlayGroundRoutes/>`.
//
// 🔴 Keep the two route lists in sync. The top level is deliberately tiny (`/`,
// `/onboarding`, `<VRoutes/>`) — real routes live inside `<VRoutes/>`, so this
// rarely changes; a new *top-level* route must be added to BOTH.
#[cfg(debug_assertions)]
#[component(transparent)]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/") view=EntryPage />
                <Route path=path!("/onboarding") view=VaultSetup />
                <VRoutes />
                <PlayGroundRoutes />
            </Routes>
        </Router>
    }
}

#[cfg(not(debug_assertions))]
#[component(transparent)]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/") view=EntryPage />
                <Route path=path!("/onboarding") view=VaultSetup />
                <VRoutes />
            </Routes>
        </Router>
    }
}
