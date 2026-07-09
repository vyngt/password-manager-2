use crate::i18n::{t_string, use_i18n};
use leptos::prelude::*;
use vedge_ui::components::Input;
use vedge_ui::components::form::helper_text::HelperText;
use vedge_ui::components::form::label::Label;
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

#[component]
pub fn HelperTextPage() -> impl IntoView {
    let i18n = use_i18n();
    let err = Signal::derive(move || t_string!(i18n, playground.error_label).to_owned());
    let suc = Signal::derive(move || t_string!(i18n, playground.success_label).to_owned());
    let wrn = Signal::derive(move || t_string!(i18n, playground.warning_label).to_owned());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"HelperText"</h1>

            <Section title="Status">
                <div class="space-y-3 max-w-md">
                    <HelperText
                        id="ht-hint"
                        message=Signal::stored("Must be at least 8 characters".to_owned())
                    />
                    <HelperText
                        id="ht-error"
                        status=Status::Error
                        message=Signal::stored("Password is too short".to_owned())
                        error_label=err
                    />
                    <HelperText
                        id="ht-success"
                        status=Status::Success
                        message=Signal::stored("Username is available".to_owned())
                        success_label=suc
                    />
                    <HelperText
                        id="ht-warning"
                        status=Status::Warning
                        message=Signal::stored("This action cannot be undone".to_owned())
                        warning_label=wrn
                    />
                </div>
            </Section>

            <Section title="Label + Input + HelperText">
                <div class="space-y-4 max-w-md">
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="ht-comp-default">"Username"</Label>
                        <Input
                            id="ht-comp-default"
                            placeholder=Signal::stored("Enter username".to_owned())
                            aria_describedby="ht-comp-default-hint"
                        />
                        <HelperText
                            id="ht-comp-default-hint"
                            message=Signal::stored(
                                "Letters, numbers, and underscores only".to_owned(),
                            )
                        />
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="ht-comp-error" status=Status::Error required=true>
                            "Email"
                        </Label>
                        <Input
                            id="ht-comp-error"
                            status=Status::Error
                            value=Signal::stored("not-an-email".to_owned())
                            aria_describedby="ht-comp-error-msg"
                        />
                        <HelperText
                            id="ht-comp-error-msg"
                            status=Status::Error
                            message=Signal::stored("Please enter a valid email address".to_owned())
                            error_label=err
                        />
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="ht-comp-success" status=Status::Success>
                            "Display name"
                        </Label>
                        <Input
                            id="ht-comp-success"
                            status=Status::Success
                            value=Signal::stored("CoolUser42".to_owned())
                            aria_describedby="ht-comp-success-msg"
                        />
                        <HelperText
                            id="ht-comp-success-msg"
                            status=Status::Success
                            message=Signal::stored("Looks great!".to_owned())
                            success_label=suc
                        />
                    </div>
                </div>
            </Section>
        </div>
    }
}
