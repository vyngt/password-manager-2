use leptos::prelude::*;
use vedge_ui::components::form::slider::Slider;
use vedge_ui::primitives::tokens::Size;

use super::common::Section;

#[component]
pub fn SliderPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Slider"</h1>

            <BasicSection />
            <SizesSection />
            <ValueLabelSection />
            <CustomRangeSection />
            <DecimalStepSection />
            <OnChangeVsEndSection />
            <DisabledSection />
        </div>
    }
}

#[component]
fn BasicSection() -> impl IntoView {
    let value = RwSignal::new(50.0_f64);
    let value_sig: Signal<f64> = value.into();
    view! {
        <Section title="Single value (controlled)">
            <div class="space-y-3">
                <Slider
                    value=value_sig
                    aria_label="Volume"
                    on_change=Callback::new(move |v: f64| value.set(v))
                />
                <div class="text-xs text-text-tertiary">"value: " {move || value.get()}</div>
            </div>
        </Section>
    }
}

#[component]
fn SizesSection() -> impl IntoView {
    view! {
        <Section title="Sizes">
            <div class="space-y-5">
                <Slider default_value=30.0 size=Size::Sm aria_label="Small" />
                <Slider default_value=50.0 size=Size::Md aria_label="Medium" />
                <Slider default_value=70.0 size=Size::Lg aria_label="Large" />
            </div>
        </Section>
    }
}

#[component]
fn ValueLabelSection() -> impl IntoView {
    let value = RwSignal::new(65.0_f64);
    let value_sig: Signal<f64> = value.into();
    view! {
        <Section title="Floating value label (drag to see it)">
            <Slider
                value=value_sig
                aria_label="Opacity"
                show_value_label=true
                format_label=Callback::new(|v: f64| format!("{v}%"))
                on_change=Callback::new(move |v: f64| value.set(v))
            />
        </Section>
    }
}

#[component]
fn CustomRangeSection() -> impl IntoView {
    let value = RwSignal::new(0.0_f64);
    let value_sig: Signal<f64> = value.into();
    view! {
        <Section title="Custom bounds — min=-50, max=50, step=5">
            <div class="space-y-3">
                <Slider
                    min=-50.0
                    max=50.0
                    step=5.0
                    value=value_sig
                    aria_label="Pan"
                    show_value_label=true
                    format_label=Callback::new(|v: f64| {
                        if v > 0.0 { format!("+{v}") } else { format!("{v}") }
                    })
                    on_change=Callback::new(move |v: f64| value.set(v))
                />
                <div class="text-xs text-text-tertiary">"value: " {move || value.get()}</div>
            </div>
        </Section>
    }
}

#[component]
fn DecimalStepSection() -> impl IntoView {
    let value = RwSignal::new(0.5_f64);
    let value_sig: Signal<f64> = value.into();
    view! {
        <Section title="Decimal step — min=0, max=1, step=0.1">
            <div class="space-y-3">
                <Slider
                    min=0.0
                    max=1.0
                    step=0.1
                    value=value_sig
                    aria_label="Gain"
                    show_value_label=true
                    format_label=Callback::new(|v: f64| format!("{v:.1}"))
                    on_change=Callback::new(move |v: f64| value.set(v))
                />
                <div class="text-xs text-text-tertiary">
                    "value: " {move || format!("{:.1}", value.get())}
                </div>
            </div>
        </Section>
    }
}

#[component]
fn OnChangeVsEndSection() -> impl IntoView {
    let change_count = RwSignal::new(0u32);
    let end_count = RwSignal::new(0u32);
    let last_value = RwSignal::new(20.0_f64);
    view! {
        <Section title="on_change (every tick) vs on_change_end (release / keyup)">
            <div class="space-y-3">
                <Slider
                    default_value=20.0
                    aria_label="Timeout"
                    on_change=Callback::new(move |v: f64| {
                        change_count.update(|n| *n += 1);
                        last_value.set(v);
                    })
                    on_change_end=Callback::new(move |_: f64| {
                        end_count.update(|n| *n += 1);
                    })
                />
                <div class="grid grid-cols-3 gap-4 text-xs text-text-secondary">
                    <div>"on_change: " <span class="text-text-primary">{move || change_count.get()}</span></div>
                    <div>"on_change_end: " <span class="text-text-primary">{move || end_count.get()}</span></div>
                    <div>"last value: " <span class="text-text-primary">{move || last_value.get()}</span></div>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn DisabledSection() -> impl IntoView {
    view! {
        <Section title="Disabled">
            <Slider
                default_value=40.0
                disabled=true
                aria_label="Disabled slider"
            />
        </Section>
    }
}
