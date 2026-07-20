use leptos::prelude::*;
use vedge_ui::components::radio_group::{RadioGroup, RadioOption};
use vedge_ui::primitives::tokens::Orientation;

use super::common::Section;

#[component]
pub fn RadioGroupPage() -> impl IntoView {
    let (radio_val, set_radio_val) = signal("option-1".to_string());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"RadioGroup"</h1>

            <Section title="Vertical (default)">
                <RadioGroup
                    options=vec![
                        RadioOption::new("option-1", "Option 1"),
                        RadioOption::new("option-2", "Option 2"),
                        RadioOption::new("option-3", "Option 3"),
                    ]
                    name="demo-vertical"
                    value=Signal::derive(move || radio_val.get())
                    on_change=Callback::new(move |v: String| set_radio_val.set(v))
                    aria_label="Vertical radio group"
                />
                <p class="text-xs text-text-tertiary pt-1">
                    "Selected: " {move || radio_val.get()}
                </p>
            </Section>

            <Section title="Horizontal">
                <RadioGroup
                    options=vec![
                        RadioOption::new("a", "Alpha"),
                        RadioOption::new("b", "Beta"),
                        RadioOption::new("c", "Gamma"),
                    ]
                    name="demo-horizontal"
                    default_value="a"
                    orientation=Orientation::Horizontal
                    aria_label="Horizontal radio group"
                />
            </Section>

            <Section title="Disabled">
                <RadioGroup
                    options=vec![
                        RadioOption::new("x", "First"),
                        RadioOption::new("y", "Second"),
                        RadioOption::new("z", "Third"),
                    ]
                    name="demo-disabled"
                    default_value="x"
                    disabled=true
                    aria_label="Disabled radio group"
                />
            </Section>
        </div>
    }
}
