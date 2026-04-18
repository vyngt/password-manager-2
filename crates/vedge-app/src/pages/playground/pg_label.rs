use leptos::prelude::*;
use vedge_ui::components::form::label::Label;
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

#[component]
pub fn LabelPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Label"</h1>

            <Section title="Default">
                <div class="flex flex-wrap items-start gap-6">
                    <Label html_for="demo-default">"Default label"</Label>
                    <Label html_for="demo-required" required=true>
                        "Required field"
                    </Label>
                    <Label html_for="demo-optional" optional=true>
                        "Optional field"
                    </Label>
                </div>
            </Section>

            <Section title="Status">
                <div class="flex flex-wrap items-start gap-6">
                    <Label html_for="s-default">"Default"</Label>
                    <Label html_for="s-error" status=Status::Error>
                        "Error"
                    </Label>
                    <Label html_for="s-success" status=Status::Success>
                        "Success"
                    </Label>
                    <Label html_for="s-warning" status=Status::Warning>
                        "Warning"
                    </Label>
                    <Label html_for="s-disabled" disabled=true>
                        "Disabled"
                    </Label>
                </div>
            </Section>
        </div>
    }
}
