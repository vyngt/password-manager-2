use leptos::prelude::*;
use vedge_ui::components::form::label::Label;
use vedge_ui::components::form::textarea::Textarea;
use vedge_ui::primitives::tokens::{Size, Status};

use super::common::Section;

#[component]
pub fn TextareaPage() -> impl IntoView {
    let (text_val, set_text_val) = signal(String::new());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Textarea"</h1>

            <Section title="Sizes">
                <div class="space-y-3 max-w-md">
                    <Textarea
                        id="size-sm"
                        size=Size::Sm
                        placeholder=Signal::stored("Small (12px)".to_string())
                    />
                    <Textarea
                        id="size-md"
                        placeholder=Signal::stored("Medium (14px) — default".to_string())
                    />
                    <Textarea
                        id="size-lg"
                        size=Size::Lg
                        placeholder=Signal::stored("Large (16px)".to_string())
                    />
                </div>
            </Section>

            <Section title="Status">
                <div class="space-y-3 max-w-md">
                    <Textarea
                        id="status-default"
                        placeholder=Signal::stored("Default".to_string())
                    />
                    <Textarea
                        id="status-error"
                        status=Status::Error
                        placeholder=Signal::stored("Error".to_string())
                    />
                    <Textarea
                        id="status-success"
                        status=Status::Success
                        placeholder=Signal::stored("Success".to_string())
                    />
                    <Textarea
                        id="status-warning"
                        status=Status::Warning
                        placeholder=Signal::stored("Warning".to_string())
                    />
                </div>
            </Section>

            <Section title="States">
                <div class="space-y-3 max-w-md">
                    <Textarea
                        id="state-disabled"
                        disabled=true
                        value=Signal::stored("Disabled textarea content".to_string())
                    />
                    <Textarea
                        id="state-readonly"
                        read_only=true
                        value=Signal::stored(
                            "Read-only content that can be selected and copied but not edited."
                                .to_string(),
                        )
                    />
                </div>
            </Section>

            <Section title="Auto-resize">
                <div class="space-y-3 max-w-md">
                    <Textarea
                        id="auto-resize"
                        rows=2
                        max_rows=6
                        placeholder=Signal::stored(
                            "Type to see auto-resize (2 rows min, 6 max)".to_string(),
                        )
                    />
                </div>
            </Section>

            <Section title="Interactive">
                <div class="space-y-3 max-w-md">
                    <div class="space-y-1">
                        <Label html_for="interactive-textarea" required=true>
                            "Notes"
                        </Label>
                        <Textarea
                            id="interactive-textarea"
                            placeholder=Signal::stored("Enter your notes...".to_string())
                            value=Signal::derive(move || text_val.get())
                            on_change=Callback::new(move |v: String| set_text_val.set(v))
                            required=true
                        />
                    </div>
                    <p class="text-xs text-text-tertiary">
                        "Characters: " {move || text_val.get().len().to_string()} " · Lines: "
                        {move || (text_val.get().lines().count().max(1)).to_string()}
                    </p>
                </div>
            </Section>
        </div>
    }
}
