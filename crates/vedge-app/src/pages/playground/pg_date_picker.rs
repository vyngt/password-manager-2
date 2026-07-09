use crate::i18n::{t_string, use_i18n};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use leptos::prelude::*;
use vedge_ui::components::form::date_picker::{
    DatePicker, DatePickerValue, DatePickerVariant, DateRange, YearMonth,
};
use vedge_ui::components::form::label::Label;
use vedge_ui::primitives::tokens::{Size, Status};

use super::common::Section;

fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn format_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn next_30_days() -> (NaiveDate, NaiveDate) {
    let t = today();
    (t, t + Duration::days(30))
}

fn weekends_in_month(anchor: NaiveDate) -> Vec<NaiveDate> {
    // month is 1..=12 by construction, so these are unreachable; `NaiveDate::MIN`
    // is a total fallback that avoids a panic in this demo helper.
    let first = NaiveDate::from_ymd_opt(anchor.year(), anchor.month(), 1).unwrap_or(NaiveDate::MIN);
    let last_day = if anchor.month() == 12 {
        NaiveDate::from_ymd_opt(anchor.year() + 1, 1, 1).unwrap_or(NaiveDate::MIN)
            - Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(anchor.year(), anchor.month() + 1, 1).unwrap_or(NaiveDate::MIN)
            - Duration::days(1)
    };
    let mut out = Vec::new();
    let mut d = first;
    while d <= last_day {
        match d.weekday() {
            Weekday::Sat | Weekday::Sun => out.push(d),
            _ => {}
        }
        d += Duration::days(1);
    }
    out
}

#[component]
pub fn DatePickerPage() -> impl IntoView {
    let i18n = use_i18n();

    // Interactive signals
    let single_val = RwSignal::new(DatePickerValue::Single(None));
    let range_val = RwSignal::new(DatePickerValue::Range(DateRange::default()));
    let month_val = RwSignal::new(DatePickerValue::Month(None));

    let locale_sig = RwSignal::new("en-US".to_owned());

    let (min_date, max_date) = next_30_days();
    let weekends = Signal::stored(weekends_in_month(today()));

    let single_readout = move || match single_val.get() {
        DatePickerValue::Single(Some(d)) => format_date(d),
        _ => "(none)".to_owned(),
    };
    let range_readout = move || match range_val.get() {
        DatePickerValue::Range(DateRange {
            start: Some(s),
            end: Some(e),
        }) => {
            format!("{} → {}", format_date(s), format_date(e))
        }
        DatePickerValue::Range(DateRange {
            start: Some(s),
            end: None,
        }) => {
            format!("{} → …", format_date(s))
        }
        _ => "(none)".to_owned(),
    };
    let month_readout = move || match month_val.get() {
        DatePickerValue::Month(Some(YearMonth { year, month })) => format!("{year}-{month:02}"),
        _ => "(none)".to_owned(),
    };

    let loc_sig = Signal::derive(move || locale_sig.get());

    let set_locale =
        move |code: &'static str| move |_: web_sys::MouseEvent| locale_sig.set(code.to_owned());

    let locale_btn_cls = move |code: &'static str| {
        Signal::derive(move || {
            let active = locale_sig.get() == code;
            if active {
                "px-3 py-1 text-xs font-medium rounded border border-primary bg-primary text-primary-foreground".to_owned()
            } else {
                "px-3 py-1 text-xs font-medium rounded border border-border bg-background text-text-primary hover:bg-surface-2 transition-colors".to_owned()
            }
        })
    };

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">
                {move || t_string!(i18n, date_picker.heading).to_owned()}
            </h1>

            <Section title="Variants">
                <div class="grid grid-cols-3 gap-3">
                    <div class="space-y-1">
                        <Label html_for="v-single">
                            {move || t_string!(i18n, date_picker.single_label).to_owned()}
                        </Label>
                        <DatePicker
                            id="v-single"
                            variant=DatePickerVariant::Single
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, date_picker.select_date).to_owned()
                            })
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="v-range">
                            {move || t_string!(i18n, date_picker.range_label).to_owned()}
                        </Label>
                        <DatePicker
                            id="v-range"
                            variant=DatePickerVariant::Range
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, date_picker.select_range).to_owned()
                            })
                        />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="v-month">
                            {move || t_string!(i18n, date_picker.month_label).to_owned()}
                        </Label>
                        <DatePicker
                            id="v-month"
                            variant=DatePickerVariant::Month
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, date_picker.select_month).to_owned()
                            })
                        />
                    </div>
                </div>
            </Section>

            <Section title="Sizes">
                <div class="space-y-3 max-w-md">
                    <DatePicker
                        id="size-sm"
                        size=Size::Sm
                        placeholder=Signal::stored("Small".to_owned())
                    />
                    <DatePicker id="size-md" placeholder=Signal::stored("Medium".to_owned()) />
                    <DatePicker
                        id="size-lg"
                        size=Size::Lg
                        placeholder=Signal::stored("Large".to_owned())
                    />
                </div>
            </Section>

            <Section title="Status">
                <div class="space-y-3 max-w-md">
                    <DatePicker id="s-default" placeholder=Signal::stored("Default".to_owned()) />
                    <DatePicker
                        id="s-error"
                        status=Status::Error
                        placeholder=Signal::stored("Error".to_owned())
                    />
                    <DatePicker
                        id="s-success"
                        status=Status::Success
                        placeholder=Signal::stored("Success".to_owned())
                    />
                    <DatePicker
                        id="s-warning"
                        status=Status::Warning
                        placeholder=Signal::stored("Warning".to_owned())
                    />
                </div>
            </Section>

            <Section title="Disabled">
                <div class="max-w-md">
                    <DatePicker
                        id="disabled"
                        disabled=true
                        default_value=DatePickerValue::Single(Some(today()))
                    />
                </div>
            </Section>

            <Section title="Constraints">
                <div class="space-y-4 max-w-md">
                    <div class="space-y-1">
                        <Label html_for="minmax">
                            {move || t_string!(i18n, date_picker.min_max_note).to_owned()}
                        </Label>
                        <DatePicker id="minmax" min_date=min_date max_date=max_date />
                    </div>
                    <div class="space-y-1">
                        <Label html_for="disabled-dates">
                            {move || t_string!(i18n, date_picker.disabled_dates_note).to_owned()}
                        </Label>
                        <DatePicker id="disabled-dates" disabled_dates=weekends />
                    </div>
                </div>
            </Section>

            <Section title="Interactive (controlled)">
                <div class="space-y-4 max-w-md">
                    <div class="space-y-1">
                        <Label html_for="int-single">"Single"</Label>
                        <DatePicker
                            id="int-single"
                            variant=DatePickerVariant::Single
                            value=Signal::derive(move || single_val.get())
                            on_change=Callback::new(move |v: DatePickerValue| single_val.set(v))
                        />
                        <p class="text-xs text-text-tertiary">
                            {move || t_string!(i18n, date_picker.value_prefix).to_owned()}" "
                            {single_readout}
                        </p>
                    </div>
                    <div class="space-y-1">
                        <Label html_for="int-range">"Range"</Label>
                        <DatePicker
                            id="int-range"
                            variant=DatePickerVariant::Range
                            value=Signal::derive(move || range_val.get())
                            on_change=Callback::new(move |v: DatePickerValue| range_val.set(v))
                        />
                        <p class="text-xs text-text-tertiary">
                            {move || t_string!(i18n, date_picker.value_prefix).to_owned()}" "
                            {range_readout}
                        </p>
                    </div>
                    <div class="space-y-1">
                        <Label html_for="int-month">"Month"</Label>
                        <DatePicker
                            id="int-month"
                            variant=DatePickerVariant::Month
                            value=Signal::derive(move || month_val.get())
                            on_change=Callback::new(move |v: DatePickerValue| month_val.set(v))
                        />
                        <p class="text-xs text-text-tertiary">
                            {move || t_string!(i18n, date_picker.value_prefix).to_owned()}" "
                            {month_readout}
                        </p>
                    </div>
                </div>
            </Section>

            <Section title="Locale">
                <div class="space-y-3 max-w-md">
                    <div class="flex gap-2">
                        <button class=locale_btn_cls("en-US") on:click=set_locale("en-US")>
                            "en-US"
                        </button>
                        <button class=locale_btn_cls("vi-VN") on:click=set_locale("vi-VN")>
                            "vi-VN"
                        </button>
                        <button class=locale_btn_cls("ja-JP") on:click=set_locale("ja-JP")>
                            "ja-JP"
                        </button>
                        <button class=locale_btn_cls("de-DE") on:click=set_locale("de-DE")>
                            "de-DE"
                        </button>
                    </div>
                    <DatePicker
                        id="locale-demo"
                        locale=loc_sig
                        default_value=DatePickerValue::Single(Some(today()))
                    />
                </div>
            </Section>
        </div>
    }
}
