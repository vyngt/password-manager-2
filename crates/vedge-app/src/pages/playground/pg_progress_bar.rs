use leptos::prelude::*;
use vedge_ui::components::foundation::progress_bar::ProgressBar;
use vedge_ui::primitives::tokens::{ProgressVariant, Size};

use super::common::Section;

#[component]
pub fn ProgressBarPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"ProgressBar"</h1>

            <VariantsSection />
            <SizesSection />
            <HeaderSection />
            <LiveSection />
            <ClampSection />
        </div>
    }
}

#[component]
fn VariantsSection() -> impl IntoView {
    view! {
        <Section title="Variants">
            <div class="space-y-4">
                <ProgressBar value=Signal::stored(45.0) aria_label="Default 45%" />
                <ProgressBar
                    value=Signal::stored(100.0)
                    variant=ProgressVariant::Success
                    aria_label="Success 100%"
                />
                <ProgressBar
                    value=Signal::stored(62.0)
                    variant=ProgressVariant::Danger
                    aria_label="Danger 62% (errored)"
                />
            </div>
        </Section>
    }
}

#[component]
fn SizesSection() -> impl IntoView {
    view! {
        <Section title="Sizes">
            <div class="space-y-4">
                <ProgressBar value=Signal::stored(60.0) size=Size::Sm aria_label="Small 60%" />
                <ProgressBar value=Signal::stored(60.0) size=Size::Md aria_label="Medium 60%" />
                <ProgressBar value=Signal::stored(60.0) size=Size::Lg aria_label="Large 60%" />
            </div>
        </Section>
    }
}

#[component]
fn HeaderSection() -> impl IntoView {
    view! {
        <Section title="Header (label + value)">
            <div class="space-y-4">
                <ProgressBar
                    value=Signal::stored(33.0)
                    label="Uploading vault.bin"
                    show_value=true
                />
                <ProgressBar
                    value=Signal::stored(80.0)
                    label="Encoding steganography payload"
                    show_value=true
                    variant=ProgressVariant::Success
                />
                <ProgressBar value=Signal::stored(15.0) label="Syncing" />
                <ProgressBar
                    value=Signal::stored(72.0)
                    show_value=true
                    aria_label="Background progress 72%"
                />
            </div>
        </Section>
    }
}

#[component]
fn LiveSection() -> impl IntoView {
    let value = RwSignal::new(25.0_f64);
    let variant = RwSignal::new(ProgressVariant::Default);

    view! {
        <Section title="Live — drag slider to see the fill transition">
            <div class="space-y-3">
                {move || {
                    view! {
                        <ProgressBar
                            value=value
                            variant=variant.get()
                            label="Live progress"
                            show_value=true
                        />
                    }
                }}
                <input
                    type="range"
                    min="0"
                    max="100"
                    step="1"
                    prop:value=move || value.get().to_string()
                    on:input:target=move |ev| {
                        if let Ok(v) = ev.target().value().parse::<f64>() {
                            value.set(v);
                            if v >= 100.0 {
                                variant.set(ProgressVariant::Success);
                            } else {
                                variant.set(ProgressVariant::Default);
                            }
                        }
                    }
                    class="w-full"
                /> <div class="flex gap-2">
                    <button
                        class="px-2 py-1 text-xs rounded border border-border bg-background text-text-primary hover:bg-surface-2"
                        on:click=move |_| {
                            value.set(0.0);
                            variant.set(ProgressVariant::Default);
                        }
                    >
                        "reset"
                    </button>
                    <button
                        class="px-2 py-1 text-xs rounded border border-border bg-background text-text-primary hover:bg-surface-2"
                        on:click=move |_| {
                            value.set(100.0);
                            variant.set(ProgressVariant::Success);
                        }
                    >
                        "complete"
                    </button>
                    <button
                        class="px-2 py-1 text-xs rounded border border-border bg-background text-text-primary hover:bg-surface-2"
                        on:click=move |_| {
                            variant.set(ProgressVariant::Danger);
                        }
                    >
                        "mark error"
                    </button>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn ClampSection() -> impl IntoView {
    view! {
        <Section title="Clamp (values outside 0\u{2013}100 are clamped; dev-assertion logs)">
            <div class="space-y-4">
                <ProgressBar
                    value=Signal::stored(-20.0)
                    label="value = -20 (clamped to 0)"
                    show_value=true
                />
                <ProgressBar
                    value=Signal::stored(150.0)
                    label="value = 150 (clamped to 100)"
                    show_value=true
                    variant=ProgressVariant::Success
                />
            </div>
        </Section>
    }
}
