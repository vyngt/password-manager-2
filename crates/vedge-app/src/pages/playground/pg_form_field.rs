use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::Input;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

#[component]
pub fn FormFieldPage() -> impl IntoView {
    let i18n = use_i18n();
    let err = Signal::derive(move || t_string!(i18n, playground.error_label).to_string());
    let suc = Signal::derive(move || t_string!(i18n, playground.success_label).to_string());
    let wrn = Signal::derive(move || t_string!(i18n, playground.warning_label).to_string());

    // Controlled demo: value reactivity while status stays at Default.
    let (name, set_name) = signal(String::new());
    let name_hint = Signal::derive(move || {
        let v = name.get();
        if v.is_empty() {
            "Start typing to see the live helper update".to_string()
        } else {
            format!("{} characters", v.len())
        }
    });

    view! {
        <div class="p-6 max-w-2xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Form Field"</h1>

            <Section title="Basic — label + input + hint">
                <FormField
                    label="Vault name"
                    id="ff-basic"
                    hint="Used to identify this vault in the sidebar"
                >
                    <Input
                        id="ff-basic"
                        aria_describedby="ff-basic-helper"
                        placeholder=Signal::stored("e.g. Personal".to_string())
                    />
                </FormField>
            </Section>

            <Section title="Required + error">
                <FormField
                    label="Email"
                    id="ff-email"
                    required=true
                    status=Status::Error
                    error="Please enter a valid email address"
                    error_label=err
                >
                    <Input
                        id="ff-email"
                        status=Status::Error
                        value=Signal::stored("not-an-email".to_string())
                        aria_describedby="ff-email-helper"
                    />
                </FormField>
            </Section>

            <Section title="Success">
                <FormField
                    label="Display name"
                    id="ff-success"
                    status=Status::Success
                    success_message="Available"
                    success_label=suc
                >
                    <Input
                        id="ff-success"
                        status=Status::Success
                        value=Signal::stored("CoolUser42".to_string())
                        aria_describedby="ff-success-helper"
                    />
                </FormField>
            </Section>

            <Section title="Warning">
                <FormField
                    label="Expiry"
                    id="ff-warning"
                    status=Status::Warning
                    warning="This vault expires in under 24 hours"
                    warning_label=wrn
                >
                    <Input
                        id="ff-warning"
                        status=Status::Warning
                        value=Signal::stored("2026-04-19".to_string())
                        aria_describedby="ff-warning-helper"
                    />
                </FormField>
            </Section>

            <Section title="Disabled">
                <FormField
                    label="Machine ID"
                    id="ff-disabled"
                    disabled=true
                    hint="Read-only — set at install time"
                >
                    <Input
                        id="ff-disabled"
                        disabled=true
                        value=Signal::stored("v-3f8a-9b21".to_string())
                        aria_describedby="ff-disabled-helper"
                    />
                </FormField>
            </Section>

            <Section title="No hint — no helper row rendered">
                <FormField label="Notes" id="ff-no-hint">
                    <Input id="ff-no-hint" placeholder=Signal::stored("Optional".to_string()) />
                </FormField>
            </Section>

            <Section title="Interactive — reactive hint">
                <FormField label="Pick a name" id="ff-interactive" required=true hint=name_hint>
                    <Input
                        id="ff-interactive"
                        value=Signal::derive(move || name.get())
                        on_input=Callback::new(move |v: String| set_name.set(v))
                        aria_describedby="ff-interactive-helper"
                    />
                </FormField>
                <div class="mt-2 text-xs text-text-tertiary">
                    "Hint text is a reactive TextProp. Status remains Default — the codebase uses static Status across all form controls."
                </div>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Consumer wiring: pass matching "<code>"id"</code>" and "
                <code>"aria-describedby=\"{id}-helper\""</code>
                " to the control. FormField cannot inject props into the slot."
            </div>
        </div>
    }
}
