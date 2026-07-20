//! Generator route (`/v/generator`) — page chrome around the reusable
//! [`GeneratorPanel`]. Mirrors `SettingsPage`'s centered scroll layout.

use crate::features::generator::generator_panel::GeneratorPanel;
use crate::i18n::{t, use_i18n};
use leptos::prelude::*;

#[component]
pub fn GeneratorPage() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        // Pattern B: the title header stays fixed while the panel owns the remaining
        // height and scrolls. The panel is reusable (also embedded in the entry-form
        // dialog), so the scroll band lives at the page level, not inside the panel.
        <div class="h-full p-6 flex flex-col">
            <div class="max-w-xl w-full mx-auto flex flex-col flex-1 min-h-0 gap-6">
                <div class="shrink-0">
                    <h1 class="text-xl font-semibold text-text-primary">
                        {move || t!(i18n, generator.title)}
                    </h1>
                    <p class="text-sm text-text-secondary mt-1">
                        {move || t!(i18n, generator.subtitle)}
                    </p>
                </div>
                <div class="flex-1 min-h-0 overflow-y-auto">
                    <GeneratorPanel />
                </div>
            </div>
        </div>
    }
}
