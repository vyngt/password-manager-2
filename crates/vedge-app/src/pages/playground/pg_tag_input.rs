use leptos::prelude::*;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::form::tag_input::TagInput;
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

#[component]
pub fn TagInputPage() -> impl IntoView {
    let (basic, set_basic) = signal(Vec::<String>::new());
    let (paste_tags, set_paste) = signal(Vec::<String>::new());
    let (limited, set_limited) = signal(vec!["Work".to_string(), "Finance".to_string()]);
    let (err_tags, set_err) = signal(vec!["invalid tag".to_string()]);
    let (ok_tags, set_ok) = signal(vec!["approved".to_string()]);
    let (warn_tags, set_warn) = signal(vec!["pending-review".to_string()]);
    let (readonly_tags, _) = signal(vec!["locked-a".to_string(), "locked-b".to_string()]);
    let (ff_tags, set_ff) = signal(Vec::<String>::new());

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Tag Input"</h1>

            <Section title="Basic — controlled">
                <TagInput
                    value=Signal::derive(move || basic.get())
                    on_change=Callback::new(move |v: Vec<String>| set_basic.set(v))
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "Tags: "
                    <code class="text-text-primary">{move || format!("{:?}", basic.get())}</code>
                </div>
            </Section>

            <Section title="Paste support">
                <TagInput
                    value=Signal::derive(move || paste_tags.get())
                    on_change=Callback::new(move |v: Vec<String>| set_paste.set(v))
                    placeholder="Paste a comma- or newline-separated list"
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "Try pasting: "
                    <code class="text-text-primary">"apple, banana, cherry"</code>
                </div>
            </Section>

            <Section title="max_tags = 5">
                <TagInput
                    value=Signal::derive(move || limited.get())
                    on_change=Callback::new(move |v: Vec<String>| set_limited.set(v))
                    max_tags=5
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "After 5 tags the input is replaced with a hint; existing chips remain removable."
                </div>
            </Section>

            <Section title="Statuses">
                <div class="space-y-3">
                    <TagInput
                        value=Signal::derive(move || err_tags.get())
                        on_change=Callback::new(move |v: Vec<String>| set_err.set(v))
                        status=Status::Error
                    />
                    <TagInput
                        value=Signal::derive(move || ok_tags.get())
                        on_change=Callback::new(move |v: Vec<String>| set_ok.set(v))
                        status=Status::Success
                    />
                    <TagInput
                        value=Signal::derive(move || warn_tags.get())
                        on_change=Callback::new(move |v: Vec<String>| set_warn.set(v))
                        status=Status::Warning
                    />
                </div>
            </Section>

            <Section title="Disabled">
                <TagInput
                    value=Signal::derive(move || readonly_tags.get())
                    on_change=Callback::new(|_: Vec<String>| {})
                    disabled=true
                />
            </Section>

            <Section title="Inside FormField">
                <FormField
                    label="Labels"
                    id="ff-tags"
                    hint="Press Enter or comma to add. Backspace removes the last."
                >
                    <TagInput
                        id="ff-tags"
                        value=Signal::derive(move || ff_tags.get())
                        on_change=Callback::new(move |v: Vec<String>| set_ff.set(v))
                        aria_describedby="ff-tags-helper"
                    />
                </FormField>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Keyboard: Enter or comma adds a tag · Backspace on empty input removes the last · "
                "Arrow Left from empty input focuses the last chip · Arrow Left/Right navigate between chips · "
                "Delete / Backspace on a chip removes it · Escape clears typed text. "
                "Blur also commits a non-empty typed value."
            </div>
        </div>
    }
}
