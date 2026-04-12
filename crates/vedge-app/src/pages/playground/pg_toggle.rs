use leptos::prelude::*;
use vedge_ui::components::toggle::Toggle;
use vedge_ui::primitives::tokens::CheckboxSize;

use super::common::Section;

#[component]
pub fn TogglePage() -> impl IntoView {
    let (toggle_checked, set_toggle_checked) = signal(false);

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Toggle"</h1>

            <Section title="Sizes">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Toggle size=CheckboxSize::Sm aria_label="Small toggle" />
                        <span class="text-sm text-text-secondary">"Small"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle aria_label="Medium toggle" />
                        <span class="text-sm text-text-secondary">"Medium"</span>
                    </div>
                </div>
            </Section>

            <Section title="States">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Toggle aria_label="Off" />
                        <span class="text-sm text-text-secondary">"Off"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle default_checked=true aria_label="On" />
                        <span class="text-sm text-text-secondary">"On"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle disabled=true aria_label="Disabled off" />
                        <span class="text-sm text-text-secondary">"Disabled off"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle disabled=true default_checked=true aria_label="Disabled on" />
                        <span class="text-sm text-text-secondary">"Disabled on"</span>
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="flex items-center gap-2">
                    <Toggle
                        checked=Signal::derive(move || toggle_checked.get())
                        on_change=Callback::new(move |v: bool| set_toggle_checked.set(v))
                        aria_label="Interactive toggle"
                    />
                    <span class="text-sm text-text-secondary">
                        {move || if toggle_checked.get() { "On" } else { "Off" }}
                    </span>
                </div>
            </Section>
        </div>
    }
}
