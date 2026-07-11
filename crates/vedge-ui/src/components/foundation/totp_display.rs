use leptos::prelude::*;

use crate::primitives::text_prop::TextProp;
use crate::utils::text::text_or;

/// Countdown ring geometry. `viewBox` is `0 0 80 80`; the ring sits at r=34.
const RING_RADIUS: f64 = 34.0;
/// Dim the code + turn the ring amber in the final few seconds — the
/// "wait for the next code" cue.
const DIM_THRESHOLD: u32 = 5;

/// Group a numeric code into two halves for readability: `"482916"` → `"482 916"`,
/// `"12345678"` → `"1234 5678"`. The copied value stays ungrouped.
fn group_digits(code: &str) -> String {
    let mid = code.len() / 2;
    if mid == 0 || mid >= code.len() || !code.is_char_boundary(mid) {
        return code.to_owned();
    }
    let (a, b) = code.split_at(mid);
    format!("{a} {b}")
}

/// A rotating TOTP code with a circular countdown ring — **secret-agnostic**.
///
/// `vedge-ui` has no IPC and no `vedge-core` dependency, so it cannot generate
/// codes; the consumer (an app-side feature that owns the reveal) passes the
/// already-generated `code` + `seconds_remaining`, and this renders + dims. The
/// ring is driven by `stroke-dashoffset` from `seconds_remaining` (never a CSS
/// `animation: linear`, which drifts when backgrounded). `role="timer"` implies
/// `aria-live="off"` — no per-second screen-reader spam.
#[component]
pub fn TotpDisplay(
    /// The already-generated code (NOT the seed).
    #[prop(into)] code: Signal<String>,
    #[prop(into)] seconds_remaining: Signal<u32>,
    #[prop(into, default = Signal::stored(30))] period: Signal<u32>,
    /// Invoked when the copy affordance is activated. When `None`, no copy
    /// button renders (the consumer offers its own hardened copy path).
    #[prop(into, default = None)] on_copy: Option<Callback<()>>,
    #[prop(into, default = TextProp::default())] copy_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let circumference = 2.0 * std::f64::consts::PI * RING_RADIUS;
    let dash_array = circumference.to_string();

    let is_dim = Memo::new(move |_| seconds_remaining.get() <= DIM_THRESHOLD);

    let ring_offset = move || {
        let p = period.get().max(1);
        let frac = (f64::from(seconds_remaining.get()) / f64::from(p)).clamp(0.0, 1.0);
        (circumference * (1.0 - frac)).to_string()
    };

    let grouped = move || group_digits(&code.get());
    let root_cls = ["totp-display", class].join(" ");

    view! {
        <div class=root_cls>
            <div class="totp-display__ring">
                <svg
                    class="totp-display__ring-svg"
                    viewBox="0 0 80 80"
                    width="80"
                    height="80"
                    aria-hidden="true"
                >
                    <circle
                        class="totp-display__ring-track"
                        cx="40"
                        cy="40"
                        r="34"
                        fill="none"
                    />
                    <circle
                        class="totp-display__ring-fg"
                        class=("totp-display__ring-fg--warn", move || is_dim.get())
                        cx="40"
                        cy="40"
                        r="34"
                        fill="none"
                        stroke-linecap="round"
                        transform="rotate(-90 40 40)"
                        stroke-dasharray=dash_array
                        stroke-dashoffset=ring_offset
                    />
                </svg>
                <span
                    class="totp-display__code"
                    class=("totp-display__code--dim", move || is_dim.get())
                    role="timer"
                    aria-label=move || code.get()
                >
                    {grouped}
                </span>
            </div>

            {on_copy.map(|cb| {
                view! {
                    <button
                        type="button"
                        class="totp-display__copy"
                        on:click=move |_: web_sys::MouseEvent| cb.run(())
                    >
                        {move || text_or(copy_label, "Copy code")}
                    </button>
                }
            })}
        </div>
    }
}
