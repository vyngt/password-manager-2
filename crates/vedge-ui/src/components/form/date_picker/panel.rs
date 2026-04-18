use super::date_utils::{
    build_month_grid, is_disabled, is_in_month, next_month_disabled, next_year_disabled,
    prev_month_disabled, prev_year_disabled, today,
};
use super::locale::{first_day_of_week, format_month_year, month_long_names, weekday_short_names};
use super::types::{DatePickerValue, DatePickerVariant, DateRange, YearMonth};
use chrono::{Datelike, NaiveDate};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[derive(Clone, Copy)]
pub struct PanelContext {
    pub variant: DatePickerVariant,
    pub view_month: RwSignal<YearMonth>,
    pub view_year: RwSignal<i32>,
    pub focused_date: RwSignal<NaiveDate>,
    pub interim_start: RwSignal<Option<NaiveDate>>,
    pub panel_style: RwSignal<String>,
    pub data_state: RwSignal<Option<&'static str>>,
    pub panel_ref: NodeRef<leptos::html::Div>,
}

#[component]
pub fn CalendarPanel(
    ctx: PanelContext,
    locale: Signal<String>,
    effective: Memo<DatePickerValue>,
    min_date: Option<NaiveDate>,
    max_date: Option<NaiveDate>,
    #[prop(into)] disabled_dates: Signal<Vec<NaiveDate>>,
    on_select_day: Callback<NaiveDate>,
    on_select_month: Callback<YearMonth>,
    on_close: Callback<()>,
) -> impl IntoView {
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        match ev.key().as_str() {
            "Escape" => {
                ev.prevent_default();
                ev.stop_propagation();
                on_close.run(());
            }
            _ => {
                if ctx.variant == DatePickerVariant::Month {
                    handle_month_keydown(&ev, ctx, min_date, max_date, on_select_month);
                } else {
                    handle_day_keydown(
                        &ev,
                        ctx,
                        min_date,
                        max_date,
                        &disabled_dates.get_untracked(),
                        on_select_day,
                    );
                }
            }
        }
    };

    view! {
        <div
            node_ref=ctx.panel_ref
            class="datepicker-panel"
            style=move || ctx.panel_style.get()
            role="dialog"
            aria-label="Choose date"
            tabindex="-1"
            data-state=move || ctx.data_state.get()
            on:keydown=handle_keydown
        >
            {move || {
                if ctx.variant == DatePickerVariant::Month {
                    render_month_view(ctx, locale, effective, min_date, max_date, on_select_month)
                        .into_any()
                } else {
                    render_day_view(
                            ctx,
                            locale,
                            effective,
                            min_date,
                            max_date,
                            disabled_dates,
                            on_select_day,
                        )
                        .into_any()
                }
            }}
        </div>
    }
}

fn render_day_view(
    ctx: PanelContext,
    locale: Signal<String>,
    effective: Memo<DatePickerValue>,
    min_date: Option<NaiveDate>,
    max_date: Option<NaiveDate>,
    disabled_dates: Signal<Vec<NaiveDate>>,
    on_select_day: Callback<NaiveDate>,
) -> impl IntoView {
    let prev_disabled = move || prev_month_disabled(ctx.view_month.get(), min_date);
    let next_disabled = move || next_month_disabled(ctx.view_month.get(), max_date);

    let go_prev = move |_: web_sys::MouseEvent| {
        if prev_disabled() {
            return;
        }
        ctx.view_month.update(|m| *m = m.prev());
    };
    let go_next = move |_: web_sys::MouseEvent| {
        if next_disabled() {
            return;
        }
        ctx.view_month.update(|m| *m = m.next());
    };

    let header_label = move || {
        let vm = ctx.view_month.get();
        format_month_year(vm.year, vm.month, &locale.get())
    };

    let weekdays = move || {
        let loc = locale.get();
        let names = weekday_short_names(&loc);
        let fdow = first_day_of_week(&loc);
        (0..7)
            .map(|i| {
                let idx = ((fdow + i) % 7) as usize;
                names.get(idx).cloned().unwrap_or_default()
            })
            .collect::<Vec<_>>()
    };

    let cells = move || {
        let loc = locale.get();
        let fdow = first_day_of_week(&loc);
        let vm = ctx.view_month.get();
        build_month_grid(vm.year, vm.month, fdow)
    };

    view! {
        <div class="datepicker-header">
            <button
                type="button"
                class="datepicker-nav"
                aria-label="Previous month"
                disabled=move || prev_disabled()
                on:click=go_prev
            >
                <Icon icon=i::FaChevronLeftSolid />
            </button>
            <div class="datepicker-title" aria-live="polite">
                {header_label}
            </div>
            <button
                type="button"
                class="datepicker-nav"
                aria-label="Next month"
                disabled=move || next_disabled()
                on:click=go_next
            >
                <Icon icon=i::FaChevronRightSolid />
            </button>
        </div>
        <div class="datepicker-weekdays" role="row">
            {move || {
                weekdays()
                    .into_iter()
                    .map(|n| view! { <div class="datepicker-weekday">{n}</div> })
                    .collect_view()
            }}
        </div>
        <div class="day-grid" role="grid">
            {move || {
                let eff = effective.get();
                let interim = ctx.interim_start.get();
                let vm = ctx.view_month.get();
                let focused = ctx.focused_date.get();
                let dd = disabled_dates.get();
                let today_d = today();
                cells()
                    .into_iter()
                    .map(move |date| {
                        let disabled = is_disabled(date, min_date, max_date, &dd);
                        render_day_cell(
                            date,
                            vm,
                            today_d,
                            focused,
                            eff,
                            interim,
                            disabled,
                            on_select_day,
                            ctx,
                        )
                    })
                    .collect_view()
            }}
        </div>
    }
    .into_any()
}

#[allow(clippy::too_many_arguments)]
fn render_day_cell(
    date: NaiveDate,
    view_month: YearMonth,
    today_d: NaiveDate,
    focused: NaiveDate,
    effective: DatePickerValue,
    interim: Option<NaiveDate>,
    disabled: bool,
    on_select_day: Callback<NaiveDate>,
    ctx: PanelContext,
) -> impl IntoView {
    let in_month = is_in_month(date, view_month.year, view_month.month);
    let is_today = date == today_d;
    let is_focused = date == focused;

    // Range state calculation
    let (is_selected, is_range_start, is_range_end, is_in_range) = match effective {
        DatePickerValue::Single(Some(d)) => (d == date, false, false, false),
        DatePickerValue::Range(DateRange { start, end }) => {
            let eff_start = interim.or(start);
            let eff_end = if interim.is_some() { None } else { end };
            match (eff_start, eff_end) {
                (Some(s), Some(e)) => (
                    false,
                    date == s && s != e,
                    date == e && s != e,
                    date > s && date < e,
                ),
                (Some(s), None) => (date == s, false, false, false),
                _ => (false, false, false, false),
            }
        }
        _ => (false, false, false, false),
    };

    let mut classes = vec!["day-cell"];
    if !in_month {
        classes.push("day-cell--out-of-month");
    }
    if disabled {
        classes.push("day-cell--disabled");
    }
    if is_today && !is_selected && !is_range_start && !is_range_end {
        classes.push("day-cell--today");
    }
    if is_in_range && !is_range_start && !is_range_end {
        classes.push("day-cell--in-range");
    }
    if is_range_start {
        classes.push("day-cell--range-start");
    }
    if is_range_end {
        classes.push("day-cell--range-end");
    }
    if is_selected {
        classes.push("day-cell--selected");
    }
    if is_focused {
        classes.push("day-cell--focused");
    }

    let cls = classes.join(" ");
    let aria_current = if is_today { Some("date") } else { None };
    let aria_selected = if is_selected || is_range_start || is_range_end {
        Some("true")
    } else {
        None
    };
    let aria_disabled = if disabled { Some("true") } else { None };
    let tabindex = if is_focused { "0" } else { "-1" };
    let day_num = date.day();

    let on_click = move |_: web_sys::MouseEvent| {
        if disabled {
            return;
        }
        ctx.focused_date.set(date);
        on_select_day.run(date);
    };

    view! {
        <div
            class=cls
            role="gridcell"
            tabindex=tabindex
            aria-current=aria_current
            aria-selected=aria_selected
            aria-disabled=aria_disabled
            on:click=on_click
        >
            {day_num}
        </div>
    }
}

fn render_month_view(
    ctx: PanelContext,
    locale: Signal<String>,
    effective: Memo<DatePickerValue>,
    min_date: Option<NaiveDate>,
    max_date: Option<NaiveDate>,
    on_select_month: Callback<YearMonth>,
) -> impl IntoView {
    let prev_disabled = move || prev_year_disabled(ctx.view_year.get(), min_date);
    let next_disabled = move || next_year_disabled(ctx.view_year.get(), max_date);

    let go_prev = move |_: web_sys::MouseEvent| {
        if prev_disabled() {
            return;
        }
        ctx.view_year.update(|y| *y -= 1);
    };
    let go_next = move |_: web_sys::MouseEvent| {
        if next_disabled() {
            return;
        }
        ctx.view_year.update(|y| *y += 1);
    };

    let year_label = move || ctx.view_year.get().to_string();

    view! {
        <div class="datepicker-header">
            <button
                type="button"
                class="datepicker-nav"
                aria-label="Previous year"
                disabled=move || prev_disabled()
                on:click=go_prev
            >
                <Icon icon=i::FaChevronLeftSolid />
            </button>
            <div class="datepicker-title" aria-live="polite">
                {year_label}
            </div>
            <button
                type="button"
                class="datepicker-nav"
                aria-label="Next year"
                disabled=move || next_disabled()
                on:click=go_next
            >
                <Icon icon=i::FaChevronRightSolid />
            </button>
        </div>
        <div class="month-grid" role="grid">
            {move || {
                let loc = locale.get();
                let names = month_long_names(&loc);
                let year = ctx.view_year.get();
                let selected = effective.get().as_month();
                let min_y = min_date.map(|d| (d.year(), d.month()));
                let max_y = max_date.map(|d| (d.year(), d.month()));
                (1u32..=12)
                    .map(|m| {
                        let is_sel = matches!(
                            selected,
                            Some(ym)
                            if ym.year == year && ym.month == m
                        );
                        let mut disabled = false;
                        if let Some((my, mm)) = min_y {
                            if year < my || (year == my && m < mm) {
                                disabled = true;
                            }
                        }
                        if let Some((my, mm)) = max_y {
                            if year > my || (year == my && m > mm) {
                                disabled = true;
                            }
                        }
                        let mut classes = vec!["month-cell"];
                        if is_sel {
                            classes.push("month-cell--selected");
                        }
                        if disabled {
                            classes.push("month-cell--disabled");
                        }
                        let cls = classes.join(" ");
                        let name = names.get((m - 1) as usize).cloned().unwrap_or_default();
                        let on_click = move |_: web_sys::MouseEvent| {
                            if disabled {
                                return;
                            }
                            on_select_month.run(YearMonth::new(year, m));
                        };
                        let aria_selected = if is_sel { Some("true") } else { None };
                        let aria_disabled = if disabled { Some("true") } else { None };
                        view! {
                            <div
                                class=cls
                                role="gridcell"
                                tabindex="-1"
                                aria-selected=aria_selected
                                aria-disabled=aria_disabled
                                on:click=on_click
                            >
                                {name}
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
    .into_any()
}

fn handle_day_keydown(
    ev: &web_sys::KeyboardEvent,
    ctx: PanelContext,
    min_date: Option<NaiveDate>,
    max_date: Option<NaiveDate>,
    disabled_dates: &[NaiveDate],
    on_select_day: Callback<NaiveDate>,
) {
    let current = ctx.focused_date.get_untracked();
    let mut next = current;
    let mut handled = true;
    match ev.key().as_str() {
        "ArrowLeft" => next = current - chrono::Duration::days(1),
        "ArrowRight" => next = current + chrono::Duration::days(1),
        "ArrowUp" => next = current - chrono::Duration::days(7),
        "ArrowDown" => next = current + chrono::Duration::days(7),
        "PageUp" => {
            next = add_months(current, -1);
        }
        "PageDown" => {
            next = add_months(current, 1);
        }
        "Home" => {
            let dow = current.weekday().num_days_from_sunday();
            next = current - chrono::Duration::days(dow as i64);
        }
        "End" => {
            let dow = current.weekday().num_days_from_sunday();
            next = current + chrono::Duration::days(6 - dow as i64);
        }
        "Enter" | " " => {
            if !is_disabled(current, min_date, max_date, disabled_dates) {
                on_select_day.run(current);
            }
            handled = true;
        }
        _ => handled = false,
    }
    if !handled {
        return;
    }
    ev.prevent_default();
    if matches!(
        ev.key().as_str(),
        "Enter" | " "
    ) {
        return;
    }
    // Clamp to min/max bounds
    if let Some(m) = min_date {
        if next < m {
            next = m;
        }
    }
    if let Some(m) = max_date {
        if next > m {
            next = m;
        }
    }
    ctx.focused_date.set(next);
    if next.year() != ctx.view_month.get_untracked().year
        || next.month() != ctx.view_month.get_untracked().month
    {
        ctx.view_month.set(YearMonth::from_date(next));
    }
}

fn handle_month_keydown(
    ev: &web_sys::KeyboardEvent,
    ctx: PanelContext,
    min_date: Option<NaiveDate>,
    max_date: Option<NaiveDate>,
    on_select_month: Callback<YearMonth>,
) {
    let year = ctx.view_year.get_untracked();
    let cur_month = ctx.focused_date.get_untracked().month();
    let cur_year = ctx.focused_date.get_untracked().year();
    let effective_year = if cur_year == year { cur_year } else { year };

    let (mut ny, mut nm) = (effective_year, cur_month);
    let mut handled = true;
    match ev.key().as_str() {
        "ArrowLeft" => {
            if nm == 1 {
                ny -= 1;
                nm = 12;
            } else {
                nm -= 1;
            }
        }
        "ArrowRight" => {
            if nm == 12 {
                ny += 1;
                nm = 1;
            } else {
                nm += 1;
            }
        }
        "ArrowUp" => {
            if nm <= 3 {
                ny -= 1;
                nm += 9;
            } else {
                nm -= 3;
            }
        }
        "ArrowDown" => {
            if nm >= 10 {
                ny += 1;
                nm -= 9;
            } else {
                nm += 3;
            }
        }
        "PageUp" => ny -= 1,
        "PageDown" => ny += 1,
        "Enter" | " " => {
            on_select_month.run(YearMonth::new(effective_year, cur_month));
            ev.prevent_default();
            return;
        }
        _ => handled = false,
    }
    if !handled {
        return;
    }
    ev.prevent_default();
    if let Some(m) = min_date {
        if ny < m.year() || (ny == m.year() && nm < m.month()) {
            ny = m.year();
            nm = m.month();
        }
    }
    if let Some(m) = max_date {
        if ny > m.year() || (ny == m.year() && nm > m.month()) {
            ny = m.year();
            nm = m.month();
        }
    }
    ctx.view_year.set(ny);
    if let Some(d) = NaiveDate::from_ymd_opt(ny, nm, 1) {
        ctx.focused_date.set(d);
    }
}

fn add_months(date: NaiveDate, delta: i32) -> NaiveDate {
    let mut y = date.year();
    let mut m = date.month() as i32 + delta;
    while m <= 0 {
        y -= 1;
        m += 12;
    }
    while m > 12 {
        y += 1;
        m -= 12;
    }
    let m = m as u32;
    let max_day = super::date_utils::days_in_month(y, m);
    let d = date.day().min(max_day);
    NaiveDate::from_ymd_opt(y, m, d).unwrap_or(date)
}
