use leptos::prelude::*;
use vedge_ui::components::separator::Separator;
use vedge_ui::primitives::tokens::Orientation;

use super::common::Section;

#[component]
pub fn SeparatorPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Separator"</h1>

            <Section title="Horizontal">
                <div class="space-y-3">
                    <p class="text-sm text-text-secondary">"Content above"</p>
                    <Separator />
                    <p class="text-sm text-text-secondary">"Content below"</p>
                </div>
            </Section>

            <Section title="Horizontal (strong)">
                <div class="space-y-3">
                    <p class="text-sm text-text-secondary">"Content above"</p>
                    <Separator strong=true />
                    <p class="text-sm text-text-secondary">"Content below"</p>
                </div>
            </Section>

            <Section title="Labeled">
                <div class="space-y-3">
                    <Separator label="General" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                    <Separator label="Advanced" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                    <Separator label="Danger Zone" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                </div>
            </Section>

            <Section title="Vertical">
                <div class="flex items-center gap-3 h-8">
                    <span class="text-sm text-text-secondary">"Left"</span>
                    <Separator orientation=Orientation::Vertical />
                    <span class="text-sm text-text-secondary">"Right"</span>
                </div>
            </Section>
        </div>
    }
}
