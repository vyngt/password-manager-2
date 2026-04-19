use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::Input;
use vedge_ui::components::form::label::Label;
use vedge_ui::primitives::tokens::{Size, Status};

use icondata as i;
use leptos_icons::Icon;

use super::common::Section;

#[component]
pub fn InputPage() -> impl IntoView {
    let i18n = use_i18n();
    let (text_val, set_text_val) = signal(String::new());
    let (password_val, set_password_val) = signal(String::new());
    let (search_val, set_search_val) = signal(String::new());
    let (prefix_val, set_prefix_val) = signal(String::new());
    let (suffix_val, set_suffix_val) = signal(String::new());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Input"</h1>

            <Section title="Sizes">
                <div class="space-y-3 max-w-md">
                    <Input
                        id="size-sm"
                        size=Size::Sm
                        placeholder=Signal::stored("Small (28px)".to_string())
                    />
                    <Input id="size-md" placeholder=Signal::stored("Medium (36px)".to_string()) />
                    <Input
                        id="size-lg"
                        size=Size::Lg
                        placeholder=Signal::stored("Large (44px)".to_string())
                    />
                </div>
            </Section>

            <Section title="Status">
                <div class="space-y-3 max-w-md">
                    <Input id="status-default" placeholder=Signal::stored("Default".to_string()) />
                    <Input
                        id="status-error"
                        status=Status::Error
                        placeholder=Signal::stored("Error".to_string())
                    />
                    <Input
                        id="status-success"
                        status=Status::Success
                        placeholder=Signal::stored("Success".to_string())
                    />
                    <Input
                        id="status-warning"
                        status=Status::Warning
                        placeholder=Signal::stored("Warning".to_string())
                    />
                </div>
            </Section>

            <Section title="States">
                <div class="space-y-3 max-w-md">
                    <Input
                        id="state-disabled"
                        disabled=true
                        value=Signal::stored("Disabled input".to_string())
                        placeholder=Signal::stored("Disabled".to_string())
                    />
                    <Input
                        id="state-readonly"
                        read_only=true
                        value=Signal::stored("Read-only value".to_string())
                        placeholder=Signal::stored("Read-only".to_string())
                    />
                    <Input
                        id="state-loading"
                        loading=true
                        value=Signal::stored("Loading...".to_string())
                        placeholder=Signal::stored("Loading".to_string())
                    />
                </div>
            </Section>

            <Section title="Enhanced Types">
                <div class="space-y-3 max-w-md">
                    <div class="space-y-1">
                        <Label html_for="type-password">"Password (reveal toggle)"</Label>
                        <Input
                            id="type-password"
                            input_type="password"
                            placeholder=Signal::stored("Enter password".to_string())
                            value=Signal::derive(move || password_val.get())
                            on_input=Callback::new(move |v: String| set_password_val.set(v))
                            reveal_label=Signal::derive(move || {
                                t_string!(i18n, playground.show_password).to_string()
                            })
                            hide_label=Signal::derive(move || {
                                t_string!(i18n, playground.hide_password).to_string()
                            })
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="type-search">"Search (clear button)"</Label>
                        <Input
                            id="type-search"
                            input_type="search"
                            placeholder=Signal::stored("Search...".to_string())
                            value=Signal::derive(move || search_val.get())
                            on_input=Callback::new(move |v: String| set_search_val.set(v))
                            clear_label=Signal::derive(move || {
                                t_string!(i18n, playground.clear).to_string()
                            })
                        />
                    </div>
                </div>
            </Section>

            <Section title="Icons & Affixes">
                <div class="space-y-3 max-w-md">
                    <Input
                        id="icon-leading"
                        placeholder=Signal::stored("With leading icon".to_string())
                        leading_icon=Box::new(|| {
                            view! { <Icon icon=i::FaMagnifyingGlassSolid /> }.into_any()
                        })
                    />
                    <Input
                        id="icon-trailing"
                        placeholder=Signal::stored("With trailing icon".to_string())
                        trailing_icon=Box::new(|| view! { <Icon icon=i::FaGearSolid /> }.into_any())
                    />
                    <Input
                        id="affix-prefix"
                        prefix="https://"
                        placeholder=Signal::stored("example.com".to_string())
                        value=Signal::derive(move || prefix_val.get())
                        on_input=Callback::new(move |v: String| set_prefix_val.set(v))
                    />
                    <Input
                        id="affix-suffix"
                        suffix=".com"
                        placeholder=Signal::stored("domain".to_string())
                        value=Signal::derive(move || suffix_val.get())
                        on_input=Callback::new(move |v: String| set_suffix_val.set(v))
                    />
                </div>
            </Section>

            <Section title="Interactive">
                <div class="space-y-3 max-w-md">
                    <div class="space-y-1">
                        <Label html_for="interactive-text" required=true>
                            "Full name"
                        </Label>
                        <Input
                            id="interactive-text"
                            placeholder=Signal::stored("Enter your full name".to_string())
                            value=Signal::derive(move || text_val.get())
                            on_input=Callback::new(move |v: String| set_text_val.set(v))
                            required=true
                        />
                    </div>
                    <p class="text-xs text-text-tertiary">
                        "Value: "
                        {move || {
                            let v = text_val.get();
                            if v.is_empty() { "(empty)".to_string() } else { v }
                        }}
                    </p>
                </div>
            </Section>
        </div>
    }
}
