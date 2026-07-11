use std::time::Duration;

use leptos::prelude::*;
use vedge_ui::components::foundation::totp_display::TotpDisplay;

use super::common::Section;

#[component]
pub fn TotpDisplayPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"TotpDisplay"</h1>
            <p class="text-sm text-text-secondary">
                "Secret-agnostic: the consumer passes an already-generated " <code>"code"</code>
                " + "<code>"seconds_remaining"</code>
                " (vedge-ui can't generate). The ring is driven by stroke-dashoffset from the countdown."
            </p>

            <DigitsSection />
            <StatesSection />
            <PeriodSection />
            <LiveSection />
        </div>
    }
}

#[component]
fn DigitsSection() -> impl IntoView {
    view! {
        <Section title="Digit counts (6 and 8), grouped for readability">
            <div class="flex items-end gap-10">
                <div class="flex flex-col items-center gap-2">
                    <TotpDisplay
                        code=Signal::stored("482916".to_owned())
                        seconds_remaining=Signal::stored(21_u32)
                    />
                    <span class="text-xs text-text-tertiary">"6 digits"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <TotpDisplay
                        code=Signal::stored("12345678".to_owned())
                        seconds_remaining=Signal::stored(21_u32)
                    />
                    <span class="text-xs text-text-tertiary">"8 digits"</span>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn StatesSection() -> impl IntoView {
    view! {
        <Section title="States — active vs expiring (≤ 5s dims + turns amber)">
            <div class="flex items-end gap-10">
                <div class="flex flex-col items-center gap-2">
                    <TotpDisplay
                        code=Signal::stored("739104".to_owned())
                        seconds_remaining=Signal::stored(25_u32)
                    />
                    <span class="text-xs text-text-tertiary">"25s left"</span>
                </div>
                <div class="flex flex-col items-center gap-2">
                    <TotpDisplay
                        code=Signal::stored("739104".to_owned())
                        seconds_remaining=Signal::stored(3_u32)
                    />
                    <span class="text-xs text-text-tertiary">"3s left — expiring"</span>
                </div>
            </div>
        </Section>
    }
}

#[component]
fn PeriodSection() -> impl IntoView {
    view! {
        <Section title="Non-default period (60s)">
            <div class="flex flex-col items-center gap-2 w-fit">
                <TotpDisplay
                    code=Signal::stored("205837".to_owned())
                    seconds_remaining=Signal::stored(48_u32)
                    period=Signal::stored(60_u32)
                />
                <span class="text-xs text-text-tertiary">"48 / 60s"</span>
            </div>
        </Section>
    }
}

#[component]
fn LiveSection() -> impl IntoView {
    let remaining = RwSignal::new(30_u32);
    let code = RwSignal::new(String::from("482916"));
    let seed = StoredValue::new(482_916_u32);

    // Raw interval callback: only *writes* signals/StoredValue (via `.update`),
    // so it needs no reactive owner and never triggers the reactive-context
    // warning. Decrement each second; rotate the code at the boundary.
    let handle = set_interval_with_handle(
        move || {
            remaining.update(|r| {
                if *r <= 1 {
                    *r = 30;
                    seed.update_value(|s| *s = (*s + 137_913) % 1_000_000);
                    code.set(format!("{:06}", seed.get_value()));
                } else {
                    *r -= 1;
                }
            });
        },
        Duration::from_secs(1),
    );
    if let Ok(h) = handle {
        on_cleanup(move || h.clear());
    }

    view! {
        <Section title="Live — ticks each second, rotates at 0, copy fires the callback">
            <TotpDisplay
                code=Signal::derive(move || code.get())
                seconds_remaining=Signal::derive(move || remaining.get())
                on_copy=Callback::new(|()| {
                    web_sys::console::log_1(&"TotpDisplay: copy clicked".into());
                })
                copy_label="Copy code"
            />
        </Section>
    }
}
