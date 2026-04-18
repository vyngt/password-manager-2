use leptos::prelude::*;
use vedge_ui::components::foundation::step_indicator::{Step, StepIndicator};
use vedge_ui::primitives::tokens::Orientation;

use super::common::Section;

fn vault_steps() -> Vec<Step> {
    vec![
        Step::new("Create vault"),
        Step::new("Set password"),
        Step::new("Import data"),
        Step::new("Done"),
    ]
}

fn pki_steps() -> Vec<Step> {
    vec![
        Step::with_description("Generate root CA", "Create the trust anchor"),
        Step::with_description("Create intermediate", "Signed by the root"),
        Step::with_description("Issue leaf cert", "Signed by the intermediate"),
        Step::with_description("Deploy", "Install on target systems"),
    ]
}

#[component]
pub fn StepIndicatorPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"StepIndicator"</h1>

            <HorizontalSection />
            <VerticalSection />
            <InteractiveSection />
            <EdgeCasesSection />
        </div>
    }
}

#[component]
fn HorizontalSection() -> impl IntoView {
    let steps = Signal::stored(vault_steps());

    view! {
        <Section title="Horizontal — progresses through the states">
            <div class="space-y-8">
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"current_step=0 (first step active)"</p>
                    <StepIndicator steps=steps current_step=0usize />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"current_step=1"</p>
                    <StepIndicator steps=steps current_step=1usize />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"current_step=2"</p>
                    <StepIndicator steps=steps current_step=2usize />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"current_step=3 (last step active)"</p>
                    <StepIndicator steps=steps current_step=3usize />
                </div>
            </div>
        </Section>
    }
}

#[component]
fn VerticalSection() -> impl IntoView {
    let steps = Signal::stored(pki_steps());

    view! {
        <Section title="Vertical — labels and descriptions right of each node">
            <div class="max-w-sm">
                <StepIndicator
                    steps=steps
                    current_step=1usize
                    variant=Orientation::Vertical
                />
            </div>
        </Section>
    }
}

#[component]
fn InteractiveSection() -> impl IntoView {
    let steps_data = vault_steps();
    let max = steps_data.len().saturating_sub(1);
    let steps = Signal::stored(steps_data);
    let current = RwSignal::new(0usize);

    view! {
        <Section title="Interactive — advance / reset current_step">
            <div class="space-y-6">
                <StepIndicator steps=steps current_step=current />
                <div class="flex gap-2 items-center flex-wrap">
                    <button
                        type="button"
                        class="px-3 py-1.5 text-sm rounded border border-border bg-background text-text-primary hover:bg-surface-2 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || current.get() == 0
                        on:click=move |_| current.update(|c| if *c > 0 { *c -= 1; })
                    >
                        "Back"
                    </button>
                    <button
                        type="button"
                        class="px-3 py-1.5 text-sm rounded border border-border bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled={move || current.get() >= max}
                        on:click=move |_| current.update(|c| if *c < max { *c += 1; })
                    >
                        "Next"
                    </button>
                    <button
                        type="button"
                        class="px-3 py-1.5 text-sm rounded border border-border bg-background text-text-primary hover:bg-surface-2"
                        on:click=move |_| current.set(0)
                    >
                        "Reset"
                    </button>
                    <span class="text-xs text-text-tertiary ml-2">
                        {move || format!("current_step = {} / {}", current.get(), max)}
                    </span>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn EdgeCasesSection() -> impl IntoView {
    let two_steps = Signal::stored(vec![Step::new("Start"), Step::new("Finish")]);
    let clamp_steps = Signal::stored(vault_steps());

    view! {
        <Section title="Edge cases">
            <div class="space-y-8">
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">"Minimum: 2 steps"</p>
                    <StepIndicator steps=two_steps current_step=1usize />
                </div>
                <div class="space-y-2">
                    <p class="text-xs text-text-tertiary">
                        "Out-of-range current_step=99 → silently clamped to last step"
                    </p>
                    <StepIndicator steps=clamp_steps current_step=99usize />
                </div>
            </div>
        </Section>
    }
}
