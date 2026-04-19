use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;
use vedge_ui::components::date_time_picker::{DateTimePicker, DateTimeValue};
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::time_picker::{TimeFormat, TimeValue};
use vedge_ui::primitives::tokens::Status;

use super::common::Section;

fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

fn format_dt(v: Option<DateTimeValue>) -> String {
    match v {
        Some(dt) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            dt.date.year(),
            dt.date.month(),
            dt.date.day(),
            dt.time.hours,
            dt.time.minutes,
        ),
        None => "None".to_string(),
    }
}

#[component]
pub fn DateTimePickerPage() -> impl IntoView {
    let basic = RwSignal::new(Some(DateTimeValue {
        date: today(),
        time: TimeValue {
            hours: 14,
            minutes: 30,
        },
    }));

    let controlled = RwSignal::<Option<DateTimeValue>>::new(None);
    let partial_null = RwSignal::<Option<DateTimeValue>>::new(None);
    let twelve_hour = RwSignal::<Option<DateTimeValue>>::new(Some(DateTimeValue {
        date: today(),
        time: TimeValue { hours: 9, minutes: 15 },
    }));
    let constrained = RwSignal::<Option<DateTimeValue>>::new(None);
    let in_form = RwSignal::<Option<DateTimeValue>>::new(None);

    let min_bound = DateTimeValue {
        date: today(),
        time: TimeValue { hours: 8, minutes: 0 },
    };
    let max_bound = DateTimeValue {
        date: today() + chrono::Duration::days(30),
        time: TimeValue {
            hours: 20,
            minutes: 0,
        },
    };

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Date Time Picker"</h1>

            <Section title="Basic — uncontrolled with default">
                <DateTimePicker
                    id="dtp-basic"
                    default_value=DateTimeValue {
                        date: today(),
                        time: TimeValue { hours: 14, minutes: 30 },
                    }
                    on_change=Callback::new(move |v: Option<DateTimeValue>| basic.set(v))
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "Current: "
                    <code class="text-text-primary">{move || format_dt(basic.get())}</code>
                </div>
            </Section>

            <Section title="Controlled">
                <DateTimePicker
                    id="dtp-controlled"
                    value=Signal::derive(move || controlled.get())
                    on_change=Callback::new(move |v: Option<DateTimeValue>| controlled.set(v))
                />
                <div class="mt-2 text-xs text-text-tertiary break-all flex items-center gap-2">
                    "Current: "
                    <code class="text-text-primary">{move || format_dt(controlled.get())}</code>
                    <button
                        type="button"
                        class="btn btn--secondary btn--sm btn--rounded"
                        on:click=move |_| {
                            controlled.set(Some(DateTimeValue {
                                date: today(),
                                time: TimeValue { hours: 12, minutes: 0 },
                            }))
                        }
                    >
                        "Set to today noon"
                    </button>
                </div>
            </Section>

            <Section title="Partial state from null">
                <DateTimePicker
                    id="dtp-partial"
                    value=Signal::derive(move || partial_null.get())
                    on_change=Callback::new(move |v: Option<DateTimeValue>| partial_null.set(v))
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "on_change only fires once BOTH date and time are picked. Current: "
                    <code class="text-text-primary">{move || format_dt(partial_null.get())}</code>
                </div>
            </Section>

            <Section title="Time format — 12h">
                <DateTimePicker
                    id="dtp-12h"
                    value=Signal::derive(move || twelve_hour.get())
                    on_change=Callback::new(move |v: Option<DateTimeValue>| twelve_hour.set(v))
                    time_format=TimeFormat::H12
                />
            </Section>

            <Section title="Min / max (today 08:00 to today+30 20:00), 15-minute step">
                <DateTimePicker
                    id="dtp-constrained"
                    value=Signal::derive(move || constrained.get())
                    on_change=Callback::new(move |v: Option<DateTimeValue>| constrained.set(v))
                    min_datetime=min_bound
                    max_datetime=max_bound
                    time_step=15
                />
                <div class="mt-2 text-xs text-text-tertiary break-all">
                    "Picked: "
                    <code class="text-text-primary">{move || format_dt(constrained.get())}</code>
                </div>
            </Section>

            <Section title="Inside FormField">
                <FormField
                    label="Schedule breach check"
                    id="dtp-ff"
                    hint="Local time"
                >
                    <DateTimePicker
                        id="dtp-ff"
                        value=Signal::derive(move || in_form.get())
                        on_change=Callback::new(move |v: Option<DateTimeValue>| in_form.set(v))
                        aria_describedby="dtp-ff-helper"
                    />
                </FormField>
            </Section>

            <Section title="Status">
                <div class="space-y-3">
                    <DateTimePicker id="dtp-error" status=Status::Error />
                    <DateTimePicker id="dtp-success" status=Status::Success />
                    <DateTimePicker id="dtp-warning" status=Status::Warning />
                </div>
            </Section>

            <Section title="Disabled">
                <DateTimePicker
                    id="dtp-disabled"
                    default_value=DateTimeValue {
                        date: today(),
                        time: TimeValue { hours: 9, minutes: 0 },
                    }
                    disabled=true
                />
            </Section>

            <div class="text-xs text-text-tertiary">
                "Click the trigger to open the Popover. Pick a date on the left, a time on the right. "
                "Escape or click outside closes. Clear button appears when a value is set."
            </div>
        </div>
    }
}
