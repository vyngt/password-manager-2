use super::types::{
    from_12h, h12_hours_first_digit, h12_hours_second_digit, h24_hours_first_digit,
    h24_hours_second_digit, is_before, minutes_first_digit, minutes_second_digit, to_12h,
    DigitOutcome, Segment, TimeFormat, TimeValue,
};
use crate::primitives::text_prop::TextProp;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn TimePicker(
    id: &'static str,
    #[prop(optional)] format: TimeFormat,
    #[prop(optional, default = 1)] step: u8,
    #[prop(into, default = None)] value: Option<Signal<Option<TimeValue>>>,
    #[prop(optional)] default_value: Option<TimeValue>,
    #[prop(into, default = None)] on_change: Option<Callback<Option<TimeValue>>>,
    #[prop(optional)] min_time: Option<TimeValue>,
    #[prop(optional)] max_time: Option<TimeValue>,
    #[prop(optional)] disabled: bool,
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(optional, default = "")] aria_labelledby: &'static str,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let internal = RwSignal::new(default_value);
    let effective: Memo<Option<TimeValue>> =
        Memo::new(move |_| value.map(|s| s.get()).unwrap_or_else(|| internal.get()));
    let active = RwSignal::<Option<Segment>>::new(None);
    let pending = RwSignal::<Option<(Segment, u8)>>::new(None);

    let step = if step == 0 { 1 } else { step };

    let hours_ref = NodeRef::<leptos::html::Span>::new();
    let minutes_ref = NodeRef::<leptos::html::Span>::new();
    let ampm_ref = NodeRef::<leptos::html::Span>::new();

    let focus_segment = move |seg: Segment| {
        let node = match seg {
            Segment::Hours => hours_ref.get(),
            Segment::Minutes => minutes_ref.get(),
            Segment::AmPm => ampm_ref.get(),
        };
        if let Some(el) = node {
            let raw: &web_sys::HtmlElement = &el;
            let _ = raw.focus();
        }
    };

    let emit = move |new: Option<TimeValue>| {
        let changed = new != effective.get_untracked();
        internal.set(new);
        if changed && pending.get_untracked().is_none() {
            if let Some(cb) = on_change {
                cb.run(new);
            }
        }
    };

    let commit_pending = move |seg: Segment| {
        if let Some((s, d)) = pending.get_untracked() {
            if s != seg {
                return;
            }
            pending.set(None);
            let current = effective.get_untracked();
            let new = match seg {
                Segment::Hours => {
                    let h24 = match format {
                        TimeFormat::H24 => d,
                        TimeFormat::H12 => {
                            let (_, is_pm) = current.map(|v| to_12h(v.hours)).unwrap_or((12, false));
                            from_12h(if d == 0 { 12 } else { d }, is_pm)
                        }
                    };
                    Some(TimeValue {
                        hours: h24,
                        minutes: current.map(|v| v.minutes).unwrap_or(0),
                    })
                }
                Segment::Minutes => Some(TimeValue {
                    hours: current.map(|v| v.hours).unwrap_or(0),
                    minutes: d,
                }),
                Segment::AmPm => current,
            };
            emit(new);
        }
    };

    let clear_segment = move |seg: Segment| {
        pending.set(None);
        match seg {
            Segment::Hours | Segment::Minutes | Segment::AmPm => emit(None),
        }
    };

    let set_hours_24 = move |new_h24: u8| {
        let current = effective.get_untracked();
        let minutes = current.map(|v| v.minutes).unwrap_or(0);
        emit(Some(TimeValue {
            hours: new_h24.min(23),
            minutes,
        }));
    };

    let set_minutes = move |new_m: u8| {
        let current = effective.get_untracked();
        let hours = current.map(|v| v.hours).unwrap_or(0);
        emit(Some(TimeValue {
            hours,
            minutes: new_m.min(59),
        }));
    };

    let set_ampm = move |is_pm: bool| {
        let current = effective.get_untracked().unwrap_or(TimeValue { hours: 0, minutes: 0 });
        let (h12, _) = to_12h(current.hours);
        emit(Some(TimeValue {
            hours: from_12h(h12, is_pm),
            minutes: current.minutes,
        }));
    };

    // ---- Hours keydown ----
    let on_hours_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                pending.set(None);
                let current = effective.get_untracked();
                let h = current.map(|v| v.hours).unwrap_or(0);
                let new_h = (h + 1) % 24;
                set_hours_24(new_h);
            }
            "ArrowDown" => {
                ev.prevent_default();
                pending.set(None);
                let current = effective.get_untracked();
                let h = current.map(|v| v.hours).unwrap_or(0);
                let new_h = if h == 0 { 23 } else { h - 1 };
                set_hours_24(new_h);
            }
            "Backspace" | "Delete" => {
                ev.prevent_default();
                clear_segment(Segment::Hours);
            }
            k if k.len() == 1 && k.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                ev.prevent_default();
                let d = k.parse::<u8>().unwrap_or(0);
                let pend = pending.get_untracked();
                match (format, pend) {
                    (TimeFormat::H24, Some((Segment::Hours, first))) => {
                        pending.set(None);
                        let h24 = h24_hours_second_digit(first, d);
                        set_hours_24(h24);
                        focus_segment(Segment::Minutes);
                    }
                    (TimeFormat::H24, _) => match h24_hours_first_digit(d) {
                        DigitOutcome::Wait(v) => {
                            pending.set(Some((Segment::Hours, v)));
                        }
                        DigitOutcome::Commit(v) => {
                            pending.set(None);
                            set_hours_24(v);
                            focus_segment(Segment::Minutes);
                        }
                    },
                    (TimeFormat::H12, Some((Segment::Hours, first))) => {
                        pending.set(None);
                        let h12 = h12_hours_second_digit(first, d);
                        let current = effective.get_untracked();
                        let is_pm = current.map(|v| to_12h(v.hours).1).unwrap_or(false);
                        let h24 = from_12h(if h12 == 0 { 12 } else { h12 }, is_pm);
                        set_hours_24(h24);
                        focus_segment(Segment::Minutes);
                    }
                    (TimeFormat::H12, _) => match h12_hours_first_digit(d) {
                        DigitOutcome::Wait(v) => {
                            pending.set(Some((Segment::Hours, v)));
                        }
                        DigitOutcome::Commit(v) => {
                            pending.set(None);
                            let current = effective.get_untracked();
                            let is_pm = current.map(|v| to_12h(v.hours).1).unwrap_or(false);
                            set_hours_24(from_12h(v, is_pm));
                            focus_segment(Segment::Minutes);
                        }
                    },
                }
            }
            _ => {}
        }
    };

    // ---- Minutes keydown ----
    let on_minutes_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                pending.set(None);
                let current = effective.get_untracked();
                let m = current.map(|v| v.minutes).unwrap_or(0);
                let new_m = ((m as u16 + step as u16) % 60) as u8;
                set_minutes(new_m);
            }
            "ArrowDown" => {
                ev.prevent_default();
                pending.set(None);
                let current = effective.get_untracked();
                let m = current.map(|v| v.minutes).unwrap_or(0);
                let new_m = if m >= step {
                    m - step
                } else {
                    60 - (step - m)
                };
                set_minutes(new_m.min(59));
            }
            "Backspace" | "Delete" => {
                ev.prevent_default();
                clear_segment(Segment::Minutes);
            }
            k if k.len() == 1 && k.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                ev.prevent_default();
                let d = k.parse::<u8>().unwrap_or(0);
                let pend = pending.get_untracked();
                match pend {
                    Some((Segment::Minutes, first)) => {
                        pending.set(None);
                        let m = minutes_second_digit(first, d);
                        set_minutes(m);
                        if format == TimeFormat::H12 {
                            focus_segment(Segment::AmPm);
                        }
                    }
                    _ => match minutes_first_digit(d) {
                        DigitOutcome::Wait(v) => {
                            pending.set(Some((Segment::Minutes, v)));
                        }
                        DigitOutcome::Commit(v) => {
                            pending.set(None);
                            set_minutes(v);
                            if format == TimeFormat::H12 {
                                focus_segment(Segment::AmPm);
                            }
                        }
                    },
                }
            }
            _ => {}
        }
    };

    // ---- AM/PM keydown ----
    let on_ampm_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "ArrowUp" | "ArrowDown" => {
                ev.prevent_default();
                let current = effective.get_untracked();
                let is_pm = current.map(|v| to_12h(v.hours).1).unwrap_or(false);
                set_ampm(!is_pm);
            }
            "a" | "A" => {
                ev.prevent_default();
                set_ampm(false);
            }
            "p" | "P" => {
                ev.prevent_default();
                set_ampm(true);
            }
            "Backspace" | "Delete" => {
                ev.prevent_default();
                clear_segment(Segment::AmPm);
            }
            _ => {}
        }
    };

    // ---- Display helpers ----
    let hours_display = move || {
        if let Some((Segment::Hours, d)) = pending.get() {
            return format!("{d}");
        }
        match effective.get() {
            Some(v) => match format {
                TimeFormat::H24 => format!("{:02}", v.hours),
                TimeFormat::H12 => {
                    let (h12, _) = to_12h(v.hours);
                    format!("{:02}", h12)
                }
            },
            None => "--".to_string(),
        }
    };

    let minutes_display = move || {
        if let Some((Segment::Minutes, d)) = pending.get() {
            return format!("{d}");
        }
        match effective.get() {
            Some(v) => format!("{:02}", v.minutes),
            None => "--".to_string(),
        }
    };

    let ampm_display = move || match effective.get() {
        Some(v) => {
            let (_, is_pm) = to_12h(v.hours);
            if is_pm { "PM" } else { "AM" }.to_string()
        }
        None => "--".to_string(),
    };

    // ---- ARIA helpers ----
    let hours_valuenow = move || match effective.get() {
        Some(v) => match format {
            TimeFormat::H24 => v.hours as i32,
            TimeFormat::H12 => to_12h(v.hours).0 as i32,
        },
        None => 0,
    };

    let hours_valuetext = move || match effective.get() {
        Some(v) => match format {
            TimeFormat::H24 => format!("{} hours", v.hours),
            TimeFormat::H12 => format!("{} hours", to_12h(v.hours).0),
        },
        None => "unset".to_string(),
    };

    let minutes_valuenow = move || effective.get().map(|v| v.minutes as i32).unwrap_or(0);

    let minutes_valuetext = move || match effective.get() {
        Some(v) => format!("{} minutes", v.minutes),
        None => "unset".to_string(),
    };

    let ampm_valuenow = move || match effective.get() {
        Some(v) => {
            if to_12h(v.hours).1 {
                1
            } else {
                0
            }
        }
        None => 0,
    };

    let ampm_valuetext = move || match effective.get() {
        Some(v) => {
            let (_, is_pm) = to_12h(v.hours);
            if is_pm { "PM" } else { "AM" }.to_string()
        }
        None => "unset".to_string(),
    };

    // ---- Out-of-range visual ----
    let is_out_of_range = Memo::new(move |_| match effective.get() {
        Some(v) => {
            let below = min_time.map(|m| is_before(v, m)).unwrap_or(false);
            let above = max_time.map(|m| is_before(m, v)).unwrap_or(false);
            below || above
        }
        None => false,
    });

    let root_cls = move || {
        [
            "timepicker-root",
            if is_out_of_range.get() { "timepicker-root--error" } else { "" },
            if disabled { "timepicker-root--disabled" } else { "" },
            class,
        ]
        .join(" ")
    };

    let segment_cls = move |seg: Segment| {
        let empty = effective.get().is_none() && pending.get().map(|(s, _)| s) != Some(seg);
        let is_active = active.get() == Some(seg);
        [
            "timepicker-segment",
            if empty { "timepicker-segment--empty" } else { "" },
            if is_active { "timepicker-segment--active" } else { "" },
        ]
        .join(" ")
    };

    let hours_max = match format {
        TimeFormat::H24 => "23",
        TimeFormat::H12 => "12",
    };
    let hours_min = match format {
        TimeFormat::H24 => "0",
        TimeFormat::H12 => "1",
    };

    let aria_label_attr = move || {
        let v = aria_label.get();
        if v.is_empty() { None } else { Some(v) }
    };
    let aria_labelledby_attr =
        if aria_labelledby.is_empty() { None } else { Some(aria_labelledby) };
    let aria_disabled_attr = if disabled { Some("true") } else { None };
    let tab_index = if disabled { "-1" } else { "0" };

    let prevent_default_mousedown = |ev: web_sys::MouseEvent| {
        ev.prevent_default();
    };

    // Focus/blur handlers — flush pending on blur (treat single digit as final value)
    let on_hours_focus = move |_: web_sys::FocusEvent| {
        active.set(Some(Segment::Hours));
    };
    let on_minutes_focus = move |_: web_sys::FocusEvent| {
        active.set(Some(Segment::Minutes));
    };
    let on_ampm_focus = move |_: web_sys::FocusEvent| {
        active.set(Some(Segment::AmPm));
    };
    let on_hours_blur = move |_: web_sys::FocusEvent| {
        commit_pending(Segment::Hours);
        if active.get_untracked() == Some(Segment::Hours) {
            active.set(None);
        }
    };
    let on_minutes_blur = move |_: web_sys::FocusEvent| {
        commit_pending(Segment::Minutes);
        if active.get_untracked() == Some(Segment::Minutes) {
            active.set(None);
        }
    };
    let on_ampm_blur = move |_: web_sys::FocusEvent| {
        if active.get_untracked() == Some(Segment::AmPm) {
            active.set(None);
        }
    };

    let show_ampm = format == TimeFormat::H12;

    view! {
        <div
            role="group"
            id=id
            class=root_cls
            aria-label=aria_label_attr
            aria-labelledby=aria_labelledby_attr
            aria-disabled=aria_disabled_attr
        >
            <span class="timepicker-icon" aria-hidden="true">
                <Icon icon=i::FaClockRegular />
            </span>
            <span
                node_ref=hours_ref
                class=move || segment_cls(Segment::Hours)
                role="spinbutton"
                tabindex=tab_index
                aria-valuemin=hours_min
                aria-valuemax=hours_max
                aria-valuenow=move || hours_valuenow().to_string()
                aria-valuetext=move || hours_valuetext()
                on:focus=on_hours_focus
                on:blur=on_hours_blur
                on:keydown=on_hours_keydown
                on:mousedown=prevent_default_mousedown
                on:click=move |_: web_sys::MouseEvent| {
                    if !disabled {
                        focus_segment(Segment::Hours);
                    }
                }
            >
                {hours_display}
            </span>
            <span class="timepicker-separator" aria-hidden="true">":"</span>
            <span
                node_ref=minutes_ref
                class=move || segment_cls(Segment::Minutes)
                role="spinbutton"
                tabindex=tab_index
                aria-valuemin="0"
                aria-valuemax="59"
                aria-valuenow=move || minutes_valuenow().to_string()
                aria-valuetext=move || minutes_valuetext()
                on:focus=on_minutes_focus
                on:blur=on_minutes_blur
                on:keydown=on_minutes_keydown
                on:mousedown=prevent_default_mousedown
                on:click=move |_: web_sys::MouseEvent| {
                    if !disabled {
                        focus_segment(Segment::Minutes);
                    }
                }
            >
                {minutes_display}
            </span>
            <Show when=move || show_ampm>
                <span
                    node_ref=ampm_ref
                    class=move || segment_cls(Segment::AmPm)
                    role="spinbutton"
                    tabindex=tab_index
                    aria-valuemin="0"
                    aria-valuemax="1"
                    aria-valuenow=move || ampm_valuenow().to_string()
                    aria-valuetext=move || ampm_valuetext()
                    on:focus=on_ampm_focus
                    on:blur=on_ampm_blur
                    on:keydown=on_ampm_keydown
                    on:mousedown=prevent_default_mousedown
                    on:click=move |_: web_sys::MouseEvent| {
                        if !disabled {
                            focus_segment(Segment::AmPm);
                        }
                    }
                >
                    {ampm_display}
                </span>
            </Show>
        </div>
    }
}
