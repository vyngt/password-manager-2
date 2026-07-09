use crate::components::feedback::popover::Popover;
use crate::components::form::date_picker::{
    DatePicker, DatePickerValue, DatePickerVariant, format_trigger_date,
};
use crate::components::form::time_picker::{TimeFormat, TimePicker, TimeValue};
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Status;
use crate::utils::id::id_with_prefix;
use chrono::NaiveDate;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

/// Combined date + time value. `hours` is always 0–23 internally regardless
/// of `time_format` display.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DateTimeValue {
    pub date: NaiveDate,
    pub time: TimeValue,
}

/// A single trigger that opens a Popover containing both a DatePicker
/// (calendar grid, inline) and a TimePicker side by side.
///
/// **Partial state**: when the controlled value is `None`, the component
/// holds internal partials for an in-progress selection. `on_change` fires
/// only once both the date and the time have been picked. In edit mode
/// (value is `Some`), any change to either sub-picker fires `on_change`
/// immediately with the updated combined value.
#[component]
pub fn DateTimePicker(
    id: &'static str,
    #[prop(into, default = None)] value: Option<Signal<Option<DateTimeValue>>>,
    #[prop(optional, default = None)] default_value: Option<DateTimeValue>,
    #[prop(into, default = None)] on_change: Option<Callback<Option<DateTimeValue>>>,
    #[prop(optional, default = None)] min_datetime: Option<DateTimeValue>,
    #[prop(optional, default = None)] max_datetime: Option<DateTimeValue>,
    #[prop(optional)] time_format: TimeFormat,
    #[prop(optional, default = 1)] time_step: u8,
    #[prop(optional)] date_variant: DatePickerVariant,
    #[prop(into, default = Signal::stored("en-US".to_string()))] locale: Signal<String>,
    #[prop(into, default = TextProp::from("Select date and time"))] placeholder: TextProp,
    #[prop(into, default = TextProp::from("Clear date and time"))] clear_label: TextProp,
    #[prop(into, default = TextProp::from("Choose date and time"))] panel_aria_label: TextProp,
    #[prop(optional, default = true)] clearable: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] status: Status,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if matches!(date_variant, DatePickerVariant::Range) {
        web_sys::console::error_1(
            &"DateTimePicker: `date_variant=Range` is not supported. Use two DateTimePicker instances for a datetime range."
                .into(),
        );
    }

    let open = RwSignal::new(false);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let internal_value = RwSignal::<Option<DateTimeValue>>::new(default_value);
    let partial_date = RwSignal::<Option<NaiveDate>>::new(None);
    let partial_time = RwSignal::<Option<TimeValue>>::new(None);

    let effective_value = Signal::derive(move || {
        value
            .map(|s| s.get())
            .unwrap_or_else(|| internal_value.get())
    });

    // Derived sub-picker signals
    let dp_value = Signal::derive(move || match effective_value.get() {
        Some(dt) => DatePickerValue::Single(Some(dt.date)),
        None => DatePickerValue::Single(partial_date.get()),
    });

    let tp_value = Signal::derive(move || match effective_value.get() {
        Some(dt) => Some(dt.time),
        None => partial_time.get(),
    });

    // Commit helper — writes to internal (uncontrolled) and fires on_change.
    let commit = move |next: Option<DateTimeValue>| {
        if value.is_none() {
            internal_value.set(next);
        }
        if let Some(cb) = on_change {
            cb.run(next);
        }
        if next.is_none() {
            partial_date.set(None);
            partial_time.set(None);
        }
    };

    let handle_date = Callback::new(move |dpv: DatePickerValue| {
        let new_date = match dpv {
            DatePickerValue::Single(Some(d)) => d,
            DatePickerValue::Month(Some(ym)) => NaiveDate::from_ymd_opt(ym.year, ym.month, 1)
                .unwrap_or_else(|| chrono::Utc::now().date_naive()),
            _ => return,
        };
        match effective_value.get_untracked() {
            Some(existing) => {
                let next = DateTimeValue {
                    date: new_date,
                    time: existing.time,
                };
                commit(Some(next));
            }
            None => {
                partial_date.set(Some(new_date));
                if let Some(t) = partial_time.get_untracked() {
                    let next = DateTimeValue {
                        date: new_date,
                        time: t,
                    };
                    commit(Some(next));
                    partial_date.set(None);
                    partial_time.set(None);
                }
            }
        }
    });

    let handle_time = Callback::new(move |tv: Option<TimeValue>| {
        let Some(new_time) = tv else {
            partial_time.set(None);
            return;
        };
        match effective_value.get_untracked() {
            Some(existing) => {
                let next = DateTimeValue {
                    date: existing.date,
                    time: new_time,
                };
                commit(Some(next));
            }
            None => {
                partial_time.set(Some(new_time));
                if let Some(d) = partial_date.get_untracked() {
                    let next = DateTimeValue {
                        date: d,
                        time: new_time,
                    };
                    commit(Some(next));
                    partial_date.set(None);
                    partial_time.set(None);
                }
            }
        }
    });

    // Pre-compute inner IDs — DatePicker/TimePicker require &'static str.
    let instance = id_with_prefix("dtp");
    let date_inner_id: &'static str = Box::leak(format!("{instance}-date").into_boxed_str());
    let time_inner_id: &'static str = Box::leak(format!("{instance}-time").into_boxed_str());

    // Flatten datetime bounds to DatePicker/TimePicker shapes, using
    // chrono::NaiveDate::{MIN,MAX} and 00:00 / 23:59 as effectively-unbounded
    // sentinels when a bound is not provided. See component doc / plan
    // risks — min_time/max_time over-constrain when the selected date
    // differs from min/max. Acceptable v1 behaviour.
    let min_date_naive = min_datetime.map(|m| m.date).unwrap_or(NaiveDate::MIN);
    let max_date_naive = max_datetime.map(|m| m.date).unwrap_or(NaiveDate::MAX);
    let min_time = min_datetime.map(|m| m.time).unwrap_or(TimeValue {
        hours: 0,
        minutes: 0,
    });
    let max_time = max_datetime.map(|m| m.time).unwrap_or(TimeValue {
        hours: 23,
        minutes: 59,
    });

    let anchor = Signal::derive(move || {
        trigger_ref
            .get()
            .map(|el| -> web_sys::HtmlElement { el.into() })
    });

    let display_text = move || {
        effective_value.get().map(|dt| {
            let loc = locale.get();
            let date_str = format_trigger_date(dt.date, &loc);
            let time_str = format_time_part(dt.time, time_format);
            format!("{date_str} · {time_str}")
        })
    };

    let has_value = Signal::derive(move || effective_value.get().is_some());

    let trigger_cls = Signal::derive(move || {
        let status_cls = match status {
            Status::Error => "datetime-trigger--error",
            Status::Success => "datetime-trigger--success",
            Status::Warning => "datetime-trigger--warning",
            Status::Default => "",
        };
        let has_clear = clearable && has_value.get();
        [
            "datetime-trigger",
            status_cls,
            if has_clear {
                "datetime-trigger--has-clear"
            } else {
                ""
            },
            class,
        ]
        .join(" ")
    });

    let handle_trigger_click = move |_: web_sys::MouseEvent| {
        if disabled {
            return;
        }
        open.update(|v| *v = !*v);
    };

    let handle_trigger_keydown = move |ev: web_sys::KeyboardEvent| {
        if disabled {
            return;
        }
        if matches!(ev.key().as_str(), "Enter" | " " | "ArrowDown") {
            ev.prevent_default();
            open.set(true);
        }
    };

    let handle_clear = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        commit(None);
    };

    let handle_close = Callback::new(move |_: ()| {
        open.set(false);
    });

    let aria_described = (!aria_describedby.is_empty()).then_some(aria_describedby);

    view! {
        <div class="datetime-root">
            <button
                node_ref=trigger_ref
                type="button"
                id=id
                class=trigger_cls
                disabled=disabled
                aria-haspopup="dialog"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                aria-describedby=aria_described
                on:click=handle_trigger_click
                on:keydown=handle_trigger_keydown
            >
                <span class="datetime-trigger__icon" aria-hidden="true">
                    <Icon icon=i::FaCalendarSolid />
                </span>
                {move || match display_text() {
                    Some(txt) => {
                        view! {
                            <span class="datetime-trigger__value" aria-live="polite">
                                {txt}
                            </span>
                        }
                            .into_any()
                    }
                    None => {
                        view! {
                            <span class="datetime-trigger__placeholder">
                                {move || placeholder.get()}
                            </span>
                        }
                            .into_any()
                    }
                }}
            </button>

            <Show when=move || clearable && has_value.get() && !disabled>
                <button
                    type="button"
                    class="datetime-trigger__clear"
                    aria-label=move || clear_label.get()
                    on:click=handle_clear
                >
                    "×"
                </button>
            </Show>

            <Popover open=Signal::derive(move || open.get()) on_close=handle_close anchor=anchor>
                <div
                    class="datetime-panel"
                    role="dialog"
                    aria-modal="false"
                    aria-label=move || panel_aria_label.get()
                >
                    <div class="datetime-panel__date">
                        <DatePicker
                            id=date_inner_id
                            variant=date_variant
                            inline=true
                            value=dp_value
                            on_change=handle_date
                            min_date=min_date_naive
                            max_date=max_date_naive
                            locale=locale
                        />
                    </div>
                    <div class="datetime-panel__divider" aria-hidden="true" />
                    <div class="datetime-panel__time">
                        <TimePicker
                            id=time_inner_id
                            format=time_format
                            step=time_step
                            value=tp_value
                            on_change=handle_time
                            min_time=min_time
                            max_time=max_time
                            disabled=disabled
                        />
                    </div>
                </div>
            </Popover>
        </div>
    }
}

fn format_time_part(time: TimeValue, format: TimeFormat) -> String {
    match format {
        TimeFormat::H24 => format!("{:02}:{:02}", time.hours, time.minutes),
        TimeFormat::H12 => {
            let (h12, period) = if time.hours == 0 {
                (12, "AM")
            } else if time.hours < 12 {
                (time.hours, "AM")
            } else if time.hours == 12 {
                (12, "PM")
            } else {
                (time.hours - 12, "PM")
            };
            format!("{}:{:02} {}", h12, time.minutes, period)
        }
    }
}
