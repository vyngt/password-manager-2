use chrono::{Datelike, NaiveDate};
use js_sys::{Array, Date, Intl::DateTimeFormat, Object, Reflect};
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::JsValue;

thread_local! {
    static WEEKDAY_CACHE: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
    static MONTH_CACHE:   RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
    static FDOW_CACHE:    RefCell<HashMap<String, u32>>         = RefCell::new(HashMap::new());
}

fn js_date(year: i32, month0: i32, day: i32) -> Date {
    Date::new_with_year_month_day(year as u32, month0, day)
}

fn make_formatter(locale: &str, option_key: &str, option_value: &str) -> Option<js_sys::Function> {
    let opts = Object::new();
    Reflect::set(
        &opts,
        &JsValue::from_str(option_key),
        &JsValue::from_str(option_value),
    )
    .ok()?;
    let locales = Array::of1(&JsValue::from_str(locale));
    let fmt = DateTimeFormat::new(&locales, &opts);
    Some(fmt.format())
}

/// Short weekday names ordered Sun..Sat (i.e. index 0 = Sunday).
/// The caller rotates this based on the locale's first-day-of-week.
pub fn weekday_short_names(locale: &str) -> Vec<String> {
    WEEKDAY_CACHE.with(|cache| {
        if let Some(v) = cache.borrow().get(locale) {
            return v.clone();
        }
        let Some(format_fn) = make_formatter(locale, "weekday", "short") else {
            return fallback_weekday_names();
        };
        // Jan 7 2024 was a Sunday (month index is 0-based in JS).
        let names: Vec<String> = (0..7)
            .map(|i| {
                let d = js_date(2024, 0, 7 + i);
                let r = format_fn
                    .call1(&JsValue::NULL, &d.into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                r
            })
            .collect();
        cache.borrow_mut().insert(locale.to_string(), names.clone());
        names
    })
}

pub fn month_long_names(locale: &str) -> Vec<String> {
    MONTH_CACHE.with(|cache| {
        if let Some(v) = cache.borrow().get(locale) {
            return v.clone();
        }
        let Some(format_fn) = make_formatter(locale, "month", "long") else {
            return fallback_month_names();
        };
        let names: Vec<String> = (0..12)
            .map(|i| {
                let d = js_date(2024, i, 15);
                let r = format_fn
                    .call1(&JsValue::NULL, &d.into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                r
            })
            .collect();
        cache.borrow_mut().insert(locale.to_string(), names.clone());
        names
    })
}

/// First day of week as 0..=6 where 0 = Sunday.
/// Uses a coarse table keyed on the region tag; defaults to Monday (1) when region is unknown.
pub fn first_day_of_week(locale: &str) -> u32 {
    FDOW_CACHE.with(|cache| {
        if let Some(v) = cache.borrow().get(locale) {
            return *v;
        }
        let region = region_from_tag(locale);
        let fdow = match region.as_deref() {
            // Sunday-first regions
            Some("US") | Some("CA") | Some("MX") | Some("JP") | Some("IL") | Some("KR")
            | Some("TW") | Some("HK") | Some("PH") | Some("ZA") | Some("BR") | Some("CO")
            | Some("VE") | Some("PE") | Some("EC") | Some("GT") | Some("DO") | Some("AR") => 0,
            // Saturday-first regions
            Some("AE") | Some("AF") | Some("BH") | Some("EG") | Some("IQ") | Some("IR")
            | Some("KW") | Some("LY") | Some("OM") | Some("QA") | Some("SA") | Some("SD")
            | Some("SY") | Some("YE") => 6,
            _ => 1,
        };
        cache.borrow_mut().insert(locale.to_string(), fdow);
        fdow
    })
}

pub fn format_trigger_date(date: NaiveDate, locale: &str) -> String {
    let opts = Object::new();
    let _ = Reflect::set(
        &opts,
        &JsValue::from_str("year"),
        &JsValue::from_str("numeric"),
    );
    let _ = Reflect::set(
        &opts,
        &JsValue::from_str("month"),
        &JsValue::from_str("short"),
    );
    let _ = Reflect::set(
        &opts,
        &JsValue::from_str("day"),
        &JsValue::from_str("numeric"),
    );
    let locales = Array::of1(&JsValue::from_str(locale));
    let fmt = DateTimeFormat::new(&locales, &opts);
    let format_fn = fmt.format();
    let d = js_date(date.year(), date.month0() as i32, date.day() as i32);
    format_fn
        .call1(&JsValue::NULL, &d.into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| date.to_string())
}

pub fn format_month_year(year: i32, month: u32, locale: &str) -> String {
    let names = month_long_names(locale);
    let idx = (month.saturating_sub(1)) as usize;
    let m = names.get(idx).cloned().unwrap_or_default();
    format!("{m} {year}")
}

fn region_from_tag(locale: &str) -> Option<String> {
    // e.g. "en-US" -> "US", "zh-Hant-TW" -> "TW"
    let parts: Vec<&str> = locale.split('-').collect();
    for p in parts.iter().rev() {
        if p.len() == 2 && p.chars().all(|c| c.is_ascii_uppercase()) {
            return Some(p.to_string());
        }
    }
    None
}

fn fallback_weekday_names() -> Vec<String> {
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn fallback_month_names() -> Vec<String> {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
