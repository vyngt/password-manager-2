use leptos::prelude::*;
use vedge_ui::components::form::label::Label;
use vedge_ui::components::form::time_picker::{TimeFormat, TimePicker, TimeValue};

use super::common::Section;

fn format_time(v: &TimeValue) -> String {
    format!("{:02}:{:02}", v.hours, v.minutes)
}

#[component]
pub fn TimePickerPage() -> impl IntoView {
    let controlled = RwSignal::<Option<TimeValue>>::new(None);
    let controlled_readout = move || match controlled.get() {
        Some(v) => format_time(&v),
        None => "(none)".to_string(),
    };

    let min_time = TimeValue { hours: 8, minutes: 0 };
    let max_time = TimeValue { hours: 17, minutes: 0 };

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"TimePicker"</h1>

            <Section title="Default (24h)">
                <div class="max-w-xs space-y-1">
                    <Label html_for="tp-default-label">"Notify at"</Label>
                    <TimePicker id="tp-default" aria_labelledby="tp-default-label" />
                </div>
            </Section>

            <Section title="12-hour format">
                <div class="max-w-xs space-y-1">
                    <Label html_for="tp-12h-label">"Meeting start"</Label>
                    <TimePicker id="tp-12h" format=TimeFormat::H12 aria_labelledby="tp-12h-label" />
                </div>
            </Section>

            <Section title="With default value">
                <div class="grid grid-cols-2 gap-3 max-w-xl">
                    <div class="space-y-1">
                        <Label html_for="tp-dv-24-label">"24h — 09:30"</Label>
                        <TimePicker
                            id="tp-dv-24"
                            default_value=TimeValue { hours: 9, minutes: 30 }
                            aria_labelledby="tp-dv-24-label"
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="tp-dv-12-label">"12h — 14:45 (2:45 PM)"</Label>
                        <TimePicker
                            id="tp-dv-12"
                            format=TimeFormat::H12
                            default_value=TimeValue {
                                hours: 14,
                                minutes: 45,
                            }
                            aria_labelledby="tp-dv-12-label"
                        />
                    </div>
                </div>
            </Section>

            <Section title="Step (minutes arrow increment)">
                <div class="grid grid-cols-3 gap-3 max-w-2xl">
                    <div class="space-y-1">
                        <Label html_for="tp-step-5-label">"step = 5"</Label>
                        <TimePicker
                            id="tp-step-5"
                            step=5
                            default_value=TimeValue { hours: 10, minutes: 0 }
                            aria_labelledby="tp-step-5-label"
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="tp-step-15-label">"step = 15"</Label>
                        <TimePicker
                            id="tp-step-15"
                            step=15
                            default_value=TimeValue { hours: 10, minutes: 0 }
                            aria_labelledby="tp-step-15-label"
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="tp-step-30-label">"step = 30"</Label>
                        <TimePicker
                            id="tp-step-30"
                            step=30
                            default_value=TimeValue { hours: 10, minutes: 0 }
                            aria_labelledby="tp-step-30-label"
                        />
                    </div>
                </div>
            </Section>

            <Section title="Disabled">
                <div class="max-w-xs space-y-1">
                    <Label html_for="tp-disabled-label">"Read-only time"</Label>
                    <TimePicker
                        id="tp-disabled"
                        disabled=true
                        default_value=TimeValue { hours: 12, minutes: 0 }
                        aria_labelledby="tp-disabled-label"
                    />
                </div>
            </Section>

            <Section title="Constraints (min 08:00, max 17:00)">
                <div class="max-w-xs space-y-1">
                    <Label html_for="tp-bounds-label">"Within work hours"</Label>
                    <TimePicker
                        id="tp-bounds"
                        min_time=min_time
                        max_time=max_time
                        default_value=TimeValue { hours: 20, minutes: 0 }
                        aria_labelledby="tp-bounds-label"
                    />
                    <p class="text-xs text-text-tertiary">
                        "Try arrow keys to move into or out of range."
                    </p>
                </div>
            </Section>

            <Section title="Interactive (controlled)">
                <div class="max-w-xs space-y-1">
                    <Label html_for="tp-controlled-label">"Controlled 12h"</Label>
                    <TimePicker
                        id="tp-controlled"
                        format=TimeFormat::H12
                        value=Signal::derive(move || controlled.get())
                        on_change=Callback::new(move |v: Option<TimeValue>| controlled.set(v))
                        aria_labelledby="tp-controlled-label"
                    />
                    <p class="text-xs text-text-tertiary">"Value (24h): "{controlled_readout}</p>
                </div>
            </Section>
        </div>
    }
}
