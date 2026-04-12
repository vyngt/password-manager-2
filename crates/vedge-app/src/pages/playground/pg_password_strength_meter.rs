use leptos::prelude::*;
use vedge_ui::components::form::label::Label;
use vedge_ui::components::form::password_strength_meter::PasswordStrengthMeter;
use vedge_ui::components::Input;

use super::common::Section;

#[component]
pub fn PasswordStrengthMeterPage() -> impl IntoView {
    let (strength_pw, set_strength_pw) = signal(String::new());
    let strength_score = Memo::new(move |_| {
        let len = strength_pw.get().len();
        match len {
            0 => 0u8,
            1..=3 => 1,
            4..=7 => 2,
            8..=11 => 3,
            _ => 4,
        }
    });

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"PasswordStrengthMeter"</h1>

            <Section title="Score Levels">
                <div class="space-y-4 max-w-sm">
                    <div class="space-y-1">
                        <span class="text-xs text-text-tertiary">"Score 0 (empty)"</span>
                        <PasswordStrengthMeter score=Signal::stored(0u8) />
                    </div>
                    <div class="space-y-1">
                        <span class="text-xs text-text-tertiary">"Score 1 (weak)"</span>
                        <PasswordStrengthMeter score=Signal::stored(1u8) />
                    </div>
                    <div class="space-y-1">
                        <span class="text-xs text-text-tertiary">"Score 2 (fair)"</span>
                        <PasswordStrengthMeter score=Signal::stored(2u8) />
                    </div>
                    <div class="space-y-1">
                        <span class="text-xs text-text-tertiary">"Score 3 (strong)"</span>
                        <PasswordStrengthMeter score=Signal::stored(3u8) />
                    </div>
                    <div class="space-y-1">
                        <span class="text-xs text-text-tertiary">"Score 4 (very strong)"</span>
                        <PasswordStrengthMeter score=Signal::stored(4u8) />
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="space-y-3 max-w-sm">
                    <div class="space-y-1">
                        <Label html_for="strength-pw">"Password"</Label>
                        <Input
                            id="strength-pw"
                            input_type="password"
                            placeholder=Signal::stored("Type to see strength".to_string())
                            value=Signal::derive(move || strength_pw.get())
                            on_input=Callback::new(move |v: String| set_strength_pw.set(v))
                        />
                    </div>
                    <PasswordStrengthMeter score=Signal::derive(move || strength_score.get()) />
                </div>
            </Section>

            <Section title="No Label">
                <div class="max-w-sm">
                    <PasswordStrengthMeter score=Signal::stored(3u8) show_label=false />
                </div>
            </Section>
        </div>
    }
}
