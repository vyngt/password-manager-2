use leptos::prelude::*;
use vedge_ui::components::copy_button::CopyButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use super::common::Section;

#[component]
pub fn CopyButtonPage() -> impl IntoView {
    let (success_count, set_success) = signal(0_u32);
    let (failure_count, set_failure) = signal(0_u32);
    let on_copy = Callback::new(move |ok: bool| {
        if ok {
            set_success.update(|n| *n += 1);
        } else {
            set_failure.update(|n| *n += 1);
        }
    });

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Copy Button"</h1>

            <Section title="Basic">
                <div class="flex items-center gap-3">
                    <span class="text-sm text-text-primary font-mono">"Hello, clipboard!"</span>
                    <CopyButton value=Signal::stored("Hello, clipboard!".to_string()) />
                </div>
            </Section>

            <Section title="With toast (no countdown)">
                <div class="flex items-center gap-3">
                    <span class="text-sm text-text-primary font-mono">"secret-token-abc123"</span>
                    <CopyButton
                        value=Signal::stored("secret-token-abc123".to_string())
                        label="Copy token"
                        show_toast=true
                    />
                </div>
            </Section>

            <Section title="With toast + clipboard countdown (30s)">
                <div class="flex items-center gap-3">
                    <span class="text-sm text-text-primary font-mono">"very-secret-password"</span>
                    <CopyButton
                        value=Signal::stored("very-secret-password".to_string())
                        label="Copy password"
                        show_toast=true
                        countdown=30
                    />
                </div>
                <div class="mt-2 text-xs text-text-tertiary">
                    "Expected toast: \"Copied · clears in 30s\"."
                </div>
            </Section>

            <Section title="Variants">
                <div class="flex items-center gap-2">
                    <CopyButton
                        value=Signal::stored("ghost".to_string())
                        label="Ghost"
                        variant=Variant::Ghost
                    />
                    <CopyButton
                        value=Signal::stored("secondary".to_string())
                        label="Secondary"
                        variant=Variant::Secondary
                    />
                    <CopyButton
                        value=Signal::stored("primary".to_string())
                        label="Primary"
                        variant=Variant::Primary
                    />
                </div>
            </Section>

            <Section title="Sizes">
                <div class="flex items-center gap-2">
                    <CopyButton
                        value=Signal::stored("sm".to_string())
                        label="Small"
                        size=Size::Sm
                    />
                    <CopyButton
                        value=Signal::stored("md".to_string())
                        label="Medium"
                        size=Size::Md
                    />
                    <CopyButton
                        value=Signal::stored("lg".to_string())
                        label="Large"
                        size=Size::Lg
                    />
                </div>
            </Section>

            <Section title="Disabled">
                <CopyButton
                    value=Signal::stored("unreachable".to_string())
                    label="Copy (disabled)"
                    disabled=true
                />
            </Section>

            <Section title="Localised (Vietnamese)">
                <CopyButton
                    value=Signal::stored("mật-khẩu-bí-mật".to_string())
                    label="Sao chép mật khẩu"
                    copied_label="Đã sao chép"
                    countdown_label="xóa sau {n}s"
                    toast_dismiss_label="Đóng"
                    show_toast=true
                    countdown=30
                />
            </Section>

            <Section title="on_copy callback">
                <div class="flex items-center gap-3">
                    <CopyButton
                        value=Signal::stored("tracked".to_string())
                        label="Copy with counter"
                        on_copy=on_copy
                    />
                    <div class="text-xs text-text-secondary">
                        "Success: "
                        <code class="text-text-primary">
                            {move || success_count.get().to_string()}
                        </code>
                        " · Failure: "
                        <code class="text-text-primary">
                            {move || failure_count.get().to_string()}
                        </code>
                    </div>
                </div>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Copy the value, observe icon Copy → Check, aria-label swap (screen reader says \"Copied\"), "
                "and the 2s revert. Clicking again during the copied state restarts the 2s timer."
            </div>
        </div>
    }
}
