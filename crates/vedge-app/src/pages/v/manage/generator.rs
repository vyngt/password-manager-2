//! Generator route (`/v/generator`) — page chrome around the reusable
//! [`GeneratorPanel`]. Mirrors `SettingsPage`'s centered scroll layout.

use crate::features::generator::generator_panel::GeneratorPanel;
use crate::i18n::{t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn GeneratorPage() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="h-full overflow-y-auto p-6">
            <div class="max-w-xl mx-auto space-y-6">
                <div>
                    <h1 class="text-xl font-semibold text-text-primary">
                        {move || t!(i18n, generator.title)}
                    </h1>
                    <p class="text-sm text-text-secondary mt-1">
                        {move || t!(i18n, generator.subtitle)}
                    </p>
                </div>
                <GeneratorPanel />
            </div>
        </div>
    }
}
