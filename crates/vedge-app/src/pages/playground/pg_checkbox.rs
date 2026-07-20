use leptos::prelude::*;
use vedge_ui::components::checkbox::Checkbox;
use vedge_ui::primitives::tokens::CheckboxSize;

use super::common::Section;

#[component]
pub fn CheckboxPage() -> impl IntoView {
    let (cb_checked, set_cb_checked) = signal(false);
    let (cb_indeterminate, _set_cb_indeterminate) = signal(true);

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Checkbox"</h1>

            <Section title="Sizes">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Checkbox size=CheckboxSize::Sm aria_label="Small checkbox" />
                        <span class="text-sm text-text-secondary">"Small"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox aria_label="Medium checkbox" />
                        <span class="text-sm text-text-secondary">"Medium"</span>
                    </div>
                </div>
            </Section>

            <Section title="States">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Checkbox aria_label="Unchecked" />
                        <span class="text-sm text-text-secondary">"Unchecked"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox default_checked=true aria_label="Checked" />
                        <span class="text-sm text-text-secondary">"Checked"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox
                            indeterminate=Signal::derive(move || cb_indeterminate.get())
                            aria_label="Indeterminate"
                        />
                        <span class="text-sm text-text-secondary">"Indeterminate"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox disabled=true aria_label="Disabled" />
                        <span class="text-sm text-text-secondary">"Disabled"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox
                            disabled=true
                            default_checked=true
                            aria_label="Disabled checked"
                        />
                        <span class="text-sm text-text-secondary">"Disabled checked"</span>
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="flex items-center gap-2">
                    <Checkbox
                        checked=Signal::derive(move || cb_checked.get())
                        on_change=Callback::new(move |v: bool| set_cb_checked.set(v))
                        aria_label="Interactive checkbox"
                    />
                    <span class="text-sm text-text-secondary">
                        {move || if cb_checked.get() { "Checked" } else { "Unchecked" }}
                    </span>
                </div>
            </Section>
        </div>
    }
}
