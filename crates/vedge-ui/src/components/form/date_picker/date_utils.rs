use super::types::YearMonth;
use chrono::{Datelike, NaiveDate};

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    (next_month - first).num_days() as u32
}

/// Builds a 6×7 grid of `NaiveDate` cells for the month starting on `first_dow`.
/// `first_dow` is 0=Sunday, 1=Monday, …, 6=Saturday.
/// Includes leading/trailing out-of-month days so the grid is always 42 cells.
pub fn build_month_grid(year: i32, month: u32, first_dow: u32) -> Vec<NaiveDate> {
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_weekday = first_of_month.weekday().num_days_from_sunday(); // 0..=6
    // How many cells before the 1st?
    let lead = (first_weekday + 7 - first_dow) % 7;
    let start = first_of_month - chrono::Duration::days(lead as i64);

    (0..42).map(|i| start + chrono::Duration::days(i)).collect()
}

pub fn is_disabled(
    date: NaiveDate,
    min: Option<NaiveDate>,
    max: Option<NaiveDate>,
    disabled_dates: &[NaiveDate],
) -> bool {
    if let Some(m) = min {
        if date < m {
            return true;
        }
    }
    if let Some(m) = max {
        if date > m {
            return true;
        }
    }
    disabled_dates.iter().any(|d| *d == date)
}

pub fn is_in_month(date: NaiveDate, year: i32, month: u32) -> bool {
    date.year() == year && date.month() == month
}

pub fn clamp_to_range(
    date: NaiveDate,
    min: Option<NaiveDate>,
    max: Option<NaiveDate>,
) -> NaiveDate {
    let mut d = date;
    if let Some(m) = min {
        if d < m {
            d = m;
        }
    }
    if let Some(m) = max {
        if d > m {
            d = m;
        }
    }
    d
}

/// Whether navigating to the previous month is blocked by `min_date`.
pub fn prev_month_disabled(view: YearMonth, min: Option<NaiveDate>) -> bool {
    let Some(min) = min else { return false };
    view.year < min.year() || (view.year == min.year() && view.month <= min.month())
}

pub fn next_month_disabled(view: YearMonth, max: Option<NaiveDate>) -> bool {
    let Some(max) = max else { return false };
    view.year > max.year() || (view.year == max.year() && view.month >= max.month())
}

pub fn prev_year_disabled(year: i32, min: Option<NaiveDate>) -> bool {
    let Some(min) = min else { return false };
    year <= min.year()
}

pub fn next_year_disabled(year: i32, max: Option<NaiveDate>) -> bool {
    let Some(max) = max else { return false };
    year >= max.year()
}
