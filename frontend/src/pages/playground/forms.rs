use leptos::prelude::*;
use ui::components::form::label::Label;
use ui::components::Input;
use ui::primitives::tokens::{Size, Status};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn FormsPage() -> impl IntoView {
    // Signals for interactive demos
    let (text_val, set_text_val) = signal(String::new());
    let (password_val, set_password_val) = signal(String::new());
    let (search_val, set_search_val) = signal(String::new());
    let (prefix_val, set_prefix_val) = signal(String::new());
    let (suffix_val, set_suffix_val) = signal(String::new());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">

            // =================================================================
            // Label
            // =================================================================
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

            // =================================================================
            // Input
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary pt-4">"Input"</h1>

            // Sizes
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

            // Status
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

            // States
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

            // Types — password & search
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
                        />
                    </div>
                </div>
            </Section>

            // Leading / trailing icons
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

            // Interactive demo
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

            // Label + Input composed (FormField pattern)
            <Section title="Label + Input Composition">
                <div class="space-y-4 max-w-md">
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="composed-default">"Username"</Label>
                        <Input
                            id="composed-default"
                            placeholder=Signal::stored("Enter username".to_string())
                        />
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="composed-error" status=Status::Error required=true>
                            "Email"
                        </Label>
                        <Input
                            id="composed-error"
                            status=Status::Error
                            value=Signal::stored("not-an-email".to_string())
                            placeholder=Signal::stored("Enter email".to_string())
                        />
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="composed-success" status=Status::Success>
                            "Password"
                        </Label>
                        <Input
                            id="composed-success"
                            input_type="password"
                            status=Status::Success
                            value=Signal::stored("strong-password-123".to_string())
                            placeholder=Signal::stored("Enter password".to_string())
                        />
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <Label html_for="composed-disabled" disabled=true>
                            "Locked field"
                        </Label>
                        <Input
                            id="composed-disabled"
                            disabled=true
                            value=Signal::stored("Cannot edit".to_string())
                        />
                    </div>
                </div>
            </Section>
        </div>
    }
}

#[component]
fn Section(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
            <h3 class="text-xs font-medium text-text-tertiary uppercase tracking-wide">{title}</h3>
            {children()}
        </div>
    }
}
