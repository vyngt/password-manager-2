use super::atoms::AtomsPage;
use super::buttons::ButtonsPage;
use super::color_picker::ColorPickerPage;
use super::forms::FormsPage;
use super::layout::PlaygroundLayout;
use super::theme::ThemePage;
use leptos::prelude::*;
use leptos_router::{MatchNestedRoutes, components::*, path};

#[component(transparent)]
pub fn PlayGroundRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/playground") view=PlaygroundLayout>
            <Route path=path!("/") view=ThemePage />
            <Route path=path!("/buttons") view=ButtonsPage />
            <Route path=path!("/forms") view=FormsPage />
            <Route path=path!("/atoms") view=AtomsPage />
            <Route path=path!("/color-picker") view=ColorPickerPage />
        </ParentRoute>
    }
    .into_inner()
}
