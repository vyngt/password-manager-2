use super::date_utils::{clamp_to_range, today};
use super::locale::format_trigger_date;
use super::panel::{CalendarPanel, PanelContext};
use super::types::{DatePickerValue, DatePickerVariant, DateRange, YearMonth};
use crate::primitives::tokens::{Size, Status};
use chrono::{Datelike, NaiveDate};
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[component]
pub fn DatePicker(
    id: &'static str,
    #[prop(optional)] variant: DatePickerVariant,
    #[prop(optional)] size: Size,
    #[prop(optional)] status: Status,
    #[prop(into, default = None)] value: Option<Signal<DatePickerValue>>,
    #[prop(optional)] default_value: Option<DatePickerValue>,
    #[prop(into, default = None)] on_change: Option<Callback<DatePickerValue>>,
    #[prop(optional)] min_date: Option<NaiveDate>,
    #[prop(optional)] max_date: Option<NaiveDate>,
    #[prop(into, default = Signal::stored(Vec::new()))] disabled_dates: Signal<Vec<NaiveDate>>,
    #[prop(into, default = None)] placeholder: Option<Signal<String>>,
    #[prop(into, default = Signal::stored("en-US".to_string()))] locale: Signal<String>,
    #[prop(optional)] disabled: bool,
    /// When `true`, renders just the calendar grid with no trigger or floating
    /// panel. Used by DateTimePicker to compose the calendar alongside a
    /// TimePicker inside a shared Popover.
    #[prop(optional)] inline: bool,
    /// `Month`-variant only: when `true`, the trigger displays as `MM/YYYY`
    /// (numeric) instead of the localized long form (e.g. `December 2030`). The
    /// calendar panel header is unaffected.
    #[prop(optional)] month_numeric: bool,
    #[prop(optional, default = "")] aria_describedby: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let initial = default_value.unwrap_or_else(|| DatePickerValue::empty_for(variant));
    let internal = RwSignal::new(initial);
    let effective: Memo<DatePickerValue> =
        Memo::new(move |_| value.map(|s| s.get()).unwrap_or_else(|| internal.get()));

    // Calendar state
    let view_month = RwSignal::new(YearMonth::from_date(today()));
    let view_year = RwSignal::new(today().year());
    let focused_date = RwSignal::new(today());
    let interim_start = RwSignal::<Option<NaiveDate>>::new(None);

    // Panel state — inline mode keeps the panel permanently visible.
    let mounted = RwSignal::new(inline);
    let data_state = RwSignal::<Option<&'static str>>::new(if inline {
        Some("open")
    } else {
        None
    });
    let panel_style = RwSignal::new(String::new());

    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();

    let show_ver = Arc::new(AtomicU32::new(0));
    let hide_ver = Arc::new(AtomicU32::new(0));

    let ctx = PanelContext {
        variant,
        view_month,
        view_year,
        focused_date,
        interim_start,
        panel_style,
        data_state,
        panel_ref,
    };

    // Position calculation
    let reposition = move || {
        if let Some(el) = trigger_ref.get_untracked() {
            let rect = el.get_bounding_client_rect();
            let viewport_height = web_sys::window()
                .and_then(|w| w.inner_height().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0);
            let panel_h = 320.0;
            let space_below = viewport_height - rect.bottom() - 8.0;
            let place_below = space_below >= panel_h || rect.top() < panel_h + 8.0;
            if place_below {
                panel_style.set(format!(
                    "left: {}px; top: {}px;",
                    rect.left(),
                    rect.bottom() + 4.0
                ));
            } else {
                panel_style.set(format!(
                    "left: {}px; bottom: {}px;",
                    rect.left(),
                    viewport_height - rect.top() + 4.0
                ));
            }
        }
    };

    // ---- Close ----
    let close_sv = show_ver.clone();
    let close_hv = hide_ver.clone();
    let do_close = Callback::new(move |()| {
        close_sv.fetch_add(1, Ordering::Relaxed);
        data_state.set(Some("closed"));
        interim_start.set(None);

        let ver = close_hv.load(Ordering::Relaxed);
        let hv = close_hv.clone();
        set_timeout(
            move || {
                if hv.load(Ordering::Relaxed) == ver {
                    mounted.set(false);
                    data_state.set(None);
                }
            },
            Duration::from_millis(100),
        );

        if let Some(el) = trigger_ref.get() {
            let _ = el.focus();
        }
    });

    // ---- Open ----
    let open_sv = show_ver.clone();
    let open_hv = hide_ver.clone();
    let do_open = Callback::new(move |()| {
        if disabled {
            return;
        }
        open_hv.fetch_add(1, Ordering::Relaxed);

        // Seed view + focus from current value (or today)
        let today_d = today();
        let focus_seed = match effective.get_untracked() {
            DatePickerValue::Single(Some(d)) => d,
            DatePickerValue::Range(DateRange { start: Some(s), .. }) => s,
            DatePickerValue::Month(Some(ym)) => {
                NaiveDate::from_ymd_opt(ym.year, ym.month, 1).unwrap_or(today_d)
            }
            _ => today_d,
        };
        let clamped = clamp_to_range(focus_seed, min_date, max_date);
        focused_date.set(clamped);
        view_month.set(YearMonth::from_date(clamped));
        view_year.set(clamped.year());

        reposition();

        data_state.set(None);
        mounted.set(true);

        let ver = open_sv.load(Ordering::Relaxed);
        let sv2 = open_sv.clone();
        set_timeout(
            move || {
                if sv2.load(Ordering::Relaxed) == ver {
                    data_state.set(Some("open"));
                }
            },
            Duration::ZERO,
        );

        set_timeout(
            move || {
                if let Some(panel) = panel_ref.get() {
                    let raw: &web_sys::HtmlElement = &panel;
                    let _ = raw.focus();
                }
            },
            Duration::from_millis(10),
        );
    });

    // ---- Select a day (single / range) ----
    let on_select_day = Callback::new(move |date: NaiveDate| {
        match variant {
            DatePickerVariant::Single => {
                let new_val = DatePickerValue::Single(Some(date));
                internal.set(new_val);
                if let Some(cb) = on_change {
                    cb.run(new_val);
                }
                if !inline {
                    do_close.run(());
                }
            }
            DatePickerVariant::Range => {
                let interim = interim_start.get_untracked();
                match interim {
                    None => {
                        let new_val = DatePickerValue::Range(DateRange {
                            start: Some(date),
                            end: None,
                        });
                        interim_start.set(Some(date));
                        internal.set(new_val);
                        if let Some(cb) = on_change {
                            cb.run(new_val);
                        }
                    }
                    Some(s) => {
                        let (start, end) = if date < s { (date, s) } else { (s, date) };
                        let new_val = DatePickerValue::Range(DateRange {
                            start: Some(start),
                            end: Some(end),
                        });
                        interim_start.set(None);
                        internal.set(new_val);
                        if let Some(cb) = on_change {
                            cb.run(new_val);
                        }
                        if !inline {
                            do_close.run(());
                        }
                    }
                }
            }
            DatePickerVariant::Month => {
                // Not triggered by day cells.
            }
        }
    });

    let on_select_month = Callback::new(move |ym: YearMonth| {
        let new_val = DatePickerValue::Month(Some(ym));
        internal.set(new_val);
        if let Some(cb) = on_change {
            cb.run(new_val);
        }
        if !inline {
            do_close.run(());
        }
    });

    // ---- Trigger class ----
    let is_open = move || mounted.get();
    let trigger_cls = move || {
        [
            "datepicker-trigger",
            size.date_picker_trigger_class(),
            status.date_picker_trigger_class(),
            if is_open() {
                "datepicker-trigger--open"
            } else {
                ""
            },
            if disabled {
                "datepicker-trigger--disabled"
            } else {
                ""
            },
            class,
        ]
        .join(" ")
    };

    let display_text = move || {
        let loc = locale.get();
        match effective.get() {
            DatePickerValue::Single(Some(d)) => Some(format_trigger_date(d, &loc)),
            DatePickerValue::Range(DateRange {
                start: Some(s),
                end: Some(e),
            }) => Some(format!(
                "{} – {}",
                format_trigger_date(s, &loc),
                format_trigger_date(e, &loc)
            )),
            DatePickerValue::Range(DateRange {
                start: Some(s),
                end: None,
            }) => Some(format_trigger_date(s, &loc)),
            DatePickerValue::Month(Some(ym)) => Some(if month_numeric {
                format!("{:02}/{}", ym.month, ym.year)
            } else {
                super::locale::format_month_year(ym.year, ym.month, &loc)
            }),
            _ => None,
        }
    };

    let placeholder_text = move || {
        placeholder
            .map(|s| s.get())
            .unwrap_or_else(|| "Select date".to_string())
    };

    let handle_trigger_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "Enter" | " " | "ArrowDown" => {
            ev.prevent_default();
            if !mounted.get_untracked() {
                do_open.run(());
            }
        }
        _ => {}
    };

    let aria_invalid = if status == Status::Error {
        Some("true")
    } else {
        None
    };
    let aria_describedby_attr = if aria_describedby.is_empty() {
        None
    } else {
        Some(aria_describedby)
    };

    if inline {
        return view! {
            <CalendarPanel
                ctx=ctx
                locale=locale
                effective=effective
                min_date=min_date
                max_date=max_date
                disabled_dates=disabled_dates
                on_select_day=on_select_day
                on_select_month=on_select_month
                on_close=do_close
                class="datepicker-panel--inline"
            />
        }
        .into_any();
    }

    view! {
        <div style="position: relative; display: inline-block; width: 100%;">
            <button
                node_ref=trigger_ref
                type="button"
                id=id
                class=trigger_cls
                disabled=disabled
                aria-haspopup="dialog"
                aria-expanded=move || is_open().to_string()
                aria-invalid=aria_invalid
                aria-describedby=aria_describedby_attr
                on:click=move |_: web_sys::MouseEvent| {
                    if disabled {
                        return;
                    }
                    if mounted.get_untracked() {
                        do_close.run(());
                    } else {
                        do_open.run(());
                    }
                }
                on:keydown=handle_trigger_keydown
            >
                <span class="datepicker-trigger__icon">
                    <Icon icon=i::FaCalendarSolid />
                </span>
                {move || {
                    match display_text() {
                        Some(text) => {
                            view! { <span class="datepicker-trigger__text">{text}</span> }
                                .into_any()
                        }
                        None => {
                            view! {
                                <span class="datepicker-trigger__text datepicker-trigger__placeholder">
                                    {placeholder_text()}
                                </span>
                            }
                                .into_any()
                        }
                    }
                }}
            </button>

            <Show when=move || mounted.get()>
                <div
                    style="position: fixed; inset: 0; z-index: 49;"
                    on:mousedown=move |_: web_sys::MouseEvent| do_close.run(())
                />
                <CalendarPanel
                    ctx=ctx
                    locale=locale
                    effective=effective
                    min_date=min_date
                    max_date=max_date
                    disabled_dates=disabled_dates
                    on_select_day=on_select_day
                    on_select_month=on_select_month
                    on_close=do_close
                />
            </Show>
        </div>
    }
    .into_any()
}
