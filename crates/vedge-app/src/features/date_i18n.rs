//! Shared i18n wiring for the `vedge-ui` `DatePicker`: the BCP-47 locale tag its
//! `Intl` formatter expects, and the calendar panel's five chrome aria-labels.
//! Both are reactive `Signal`s, so switching the app locale relocalizes an open
//! calendar in place (month names, weekday headers, first-day-of-week, and the
//! nav aria-labels) with no remount.

use leptos::prelude::*;
use leptos_i18n::I18nContext;

use crate::i18n::{Locale, t_string};

/// Map the active app locale to the BCP-47 tag the `DatePicker`'s `Intl`
/// formatter expects. `Locale::as_str()` yields bare `"en"`/`"vi"`; a bare tag
/// makes `first_day_of_week` fall through its unknown-region → Monday branch,
/// which would silently flip **en**'s week-start away from Sunday. Pin the
/// region so en stays Sunday-first and vi is Monday-first (correct for Vietnam).
pub fn locale_tag(i18n: I18nContext<Locale>) -> Signal<String> {
    Signal::derive(move || {
        match i18n.get_locale() {
            Locale::en => "en-US",
            Locale::vi => "vi-VN",
        }
        .to_owned()
    })
}

/// The five `DatePicker` panel aria-labels as reactive `Signal<String>`s — each
/// coerces into the component's `TextProp` props. Grouped so a call site wires
/// the calendar's a11y strings in one place regardless of variant (a day-variant
/// picker renders the month labels, a month-variant one the year labels).
pub struct CalendarLabels {
    pub dialog: Signal<String>,
    pub prev_month: Signal<String>,
    pub next_month: Signal<String>,
    pub prev_year: Signal<String>,
    pub next_year: Signal<String>,
}

/// Build the calendar aria-labels for the active locale. Returns lazily-derived
/// signals (no eager `t_string!` read), so it is safe to call in a component
/// body — the reads happen inside each `Signal::derive`, which has an owner.
pub fn calendar_labels(i18n: I18nContext<Locale>) -> CalendarLabels {
    CalendarLabels {
        dialog: Signal::derive(move || t_string!(i18n, calendar.dialog).to_owned()),
        prev_month: Signal::derive(move || t_string!(i18n, calendar.prev_month).to_owned()),
        next_month: Signal::derive(move || t_string!(i18n, calendar.next_month).to_owned()),
        prev_year: Signal::derive(move || t_string!(i18n, calendar.prev_year).to_owned()),
        next_year: Signal::derive(move || t_string!(i18n, calendar.next_year).to_owned()),
    }
}
