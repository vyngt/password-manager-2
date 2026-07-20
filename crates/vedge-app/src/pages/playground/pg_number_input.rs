use crate::i18n::*;
use leptos::prelude::*;
use vedge_ui::components::form::label::Label;
use vedge_ui::components::form::number_input::NumberInput;
use vedge_ui::primitives::tokens::{Size, Status};

use super::common::Section;

#[component]
pub fn NumberInputPage() -> impl IntoView {
    let i18n = use_i18n();
    let dec = Signal::derive(move || t_string!(i18n, playground.decrease).to_string());
    let inc = Signal::derive(move || t_string!(i18n, playground.increase).to_string());
    let (num_val, set_num_val) = signal(50.0_f64);

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"NumberInput"</h1>

            <Section title="Sizes">
                <div class="space-y-3 max-w-xs">
                    <NumberInput
                        id="num-sm"
                        size=Size::Sm
                        default_value=10.0
                        decrement_label=dec
                        increment_label=inc
                    />
                    <NumberInput
                        id="num-md"
                        default_value=25.0
                        decrement_label=dec
                        increment_label=inc
                    />
                    <NumberInput
                        id="num-lg"
                        size=Size::Lg
                        default_value=50.0
                        decrement_label=dec
                        increment_label=inc
                    />
                </div>
            </Section>

            <Section title="Status">
                <div class="space-y-3 max-w-xs">
                    <NumberInput
                        id="num-error"
                        status=Status::Error
                        default_value=0.0
                        decrement_label=dec
                        increment_label=inc
                    />
                    <NumberInput
                        id="num-success"
                        status=Status::Success
                        default_value=42.0
                        decrement_label=dec
                        increment_label=inc
                    />
                    <NumberInput
                        id="num-warning"
                        status=Status::Warning
                        default_value=99.0
                        decrement_label=dec
                        increment_label=inc
                    />
                </div>
            </Section>

            <Section title="Constraints">
                <div class="space-y-3 max-w-xs">
                    <div class="space-y-1">
                        <Label html_for="num-opacity">"Opacity"</Label>
                        <NumberInput
                            id="num-opacity"
                            default_value=100.0
                            min=0.0
                            max=100.0
                            step=1.0
                            unit="%"
                            decrement_label=dec
                            increment_label=inc
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="num-font">"Font size"</Label>
                        <NumberInput
                            id="num-font"
                            default_value=16.0
                            min=8.0
                            max=72.0
                            step=0.5
                            unit="px"
                            decrement_label=dec
                            increment_label=inc
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="num-port">"Port"</Label>
                        <NumberInput
                            id="num-port"
                            default_value=8080.0
                            min=1.0
                            max=65535.0
                            step=1.0
                            decrement_label=dec
                            increment_label=inc
                        />
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="space-y-3 max-w-xs">
                    <NumberInput
                        id="num-interactive"
                        value=Signal::derive(move || num_val.get())
                        min=0.0
                        max=100.0
                        on_change=Callback::new(move |v: f64| set_num_val.set(v))
                        decrement_label=dec
                        increment_label=inc
                    />
                    <p class="text-xs text-text-tertiary">
                        "Value: " {move || format!("{}", num_val.get())}
                    </p>
                </div>
            </Section>

            <Section title="Disabled">
                <div class="max-w-xs">
                    <NumberInput
                        id="num-disabled"
                        default_value=42.0
                        disabled=true
                        decrement_label=dec
                        increment_label=inc
                    />
                </div>
            </Section>
        </div>
    }
}
