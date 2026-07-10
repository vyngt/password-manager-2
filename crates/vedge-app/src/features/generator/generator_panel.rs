//! Standalone password-generator panel (slice 3.1).
//!
//! Exercises the pure [`vedge_generator`] engine with **zero IPC** — generation
//! is synchronous charset math in the renderer. The config lives in one
//! `RwSignal<RandomConfig>`; the secret in a [`Zeroizing`] container (wiped on
//! drop). The entropy meter is a `Memo` over the config, so it updates live the
//! instant a class is toggled — even before Regenerate — and can never disagree
//! with the generator (both call the engine's one entropy code path).
//!
//! **Copy** uses `CopyButton` (browser clipboard) — a deliberate placeholder:
//! slice 3.2 reroutes it through a hardened native `copy_text` so copied secrets
//! skip OS clipboard history/cloud sync. There is no OS-clipboard auto-clear on
//! this path yet (3.2). The plain `String` that reaches the DOM for display/copy
//! is outside Rust's zeroize control — a documented residual, not a regression.
//!
//! Entropy bands come from `entropy_band` (charset math) — **not** from the
//! `password_strength` (zxcvbn) scorer, which is for *user-entered* passwords and
//! is kept strictly separate per the generator-entropy research note.

use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_generator::{RandomConfig, entropy_band, generate_random, random_entropy_bits};
use vedge_ui::components::{CopyButton, IconButton, PasswordStrengthMeter, Slider, Toggle};
use vedge_ui::primitives::tokens::{Size, Variant};
use zeroize::Zeroizing;

use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn GeneratorPanel() -> impl IntoView {
    let i18n = use_i18n();

    let cfg = RwSignal::new(RandomConfig::default());
    let reveal = RwSignal::new(true);
    // Seed with a first secret at mount (default config is always valid).
    let secret = RwSignal::new(
        generate_random(&RandomConfig::default())
            .map(|g| g.secret)
            .unwrap_or_default(),
    );

    // Synchronous — pure math, no `spawn_local`. Reads the config untracked so the
    // handler doesn't subscribe to it.
    let regenerate = move |()| match generate_random(&cfg.get_untracked()) {
        Ok(g) => secret.set(g.secret),
        Err(_) => secret.set(Zeroizing::default()),
    };

    // Live entropy of the *process*. Gates the meter; the secret only changes on
    // Regenerate, but the meter tracks every config edit.
    let entropy = Memo::new(move |_| random_entropy_bits(&cfg.get()));
    let band = Signal::derive(move || entropy.get().map_or(0, entropy_band));
    let band_labels = Signal::derive(move || {
        [
            t_string!(i18n, generator.band_weak).to_owned(),
            t_string!(i18n, generator.band_fair).to_owned(),
            t_string!(i18n, generator.band_strong).to_owned(),
            t_string!(i18n, generator.band_excellent).to_owned(),
        ]
    });

    // Real secret for copy; masked-or-plain view for display (default revealed).
    let copy_value = Signal::derive(move || secret.with(|z| z.as_str().to_owned()));
    let display = Signal::derive(move || {
        secret.with(|z| {
            if reveal.get() {
                z.as_str().to_owned()
            } else {
                "•".repeat(z.chars().count())
            }
        })
    });

    view! {
        <div class="space-y-6">
            // ---- Output --------------------------------------------------
            <div class="flex items-center gap-2 rounded-md border border-border bg-primary-muted p-3">
                <code class="flex-1 select-all break-all font-mono text-sm text-text-primary min-h-[1.25rem]">
                    {move || display.get()}
                </code>
                <IconButton
                    variant=Variant::Ghost
                    size=Size::Sm
                    aria_label=Signal::derive(move || {
                        if reveal.get() {
                            t_string!(i18n, generator.hide).to_owned()
                        } else {
                            t_string!(i18n, generator.reveal).to_owned()
                        }
                    })
                    on_click=Callback::new(move |()| reveal.update(|r| *r = !*r))
                >
                    // `aria-hidden` on the wrapping span (not `attr:` on the
                    // `<Icon>`) — the latter trips the Leptos RPIT bound in a
                    // `<Show>`/returned position; the IconButton carries the name.
                    <span aria-hidden="true">
                        <Show
                            when=move || reveal.get()
                            fallback=|| view! { <Icon icon=i::FaEyeSolid /> }
                        >
                            <Icon icon=i::FaEyeSlashSolid />
                        </Show>
                    </span>
                </IconButton>
                <CopyButton
                    value=copy_value
                    show_toast=true
                    label=Signal::derive(move || t_string!(i18n, generator.copy).to_owned())
                    copied_label=Signal::derive(move || {
                        t_string!(i18n, generator.copied).to_owned()
                    })
                    toast_dismiss_label=Signal::derive(move || {
                        t_string!(i18n, generator.dismiss).to_owned()
                    })
                />
                <IconButton
                    variant=Variant::Ghost
                    size=Size::Sm
                    aria_label=Signal::derive(move || {
                        t_string!(i18n, generator.regenerate).to_owned()
                    })
                    on_click=Callback::new(regenerate)
                >
                    <span aria-hidden="true">
                        <Icon icon=i::FaArrowsRotateSolid />
                    </span>
                </IconButton>
            </div>

            // ---- Entropy meter -------------------------------------------
            <div class="space-y-1.5">
                <PasswordStrengthMeter
                    score=band
                    level_labels=band_labels
                    strength_label=Signal::derive(move || {
                        t_string!(i18n, generator.entropy).to_owned()
                    })
                />
                {move || match entropy.get() {
                    Ok(bits) => {
                        Either::Left(
                            view! {
                                <p class="text-xs text-text-secondary tabular-nums">
                                    {format!("{bits:.0} ")}
                                    {t_string!(i18n, generator.bits).to_owned()}
                                </p>
                            },
                        )
                    }
                    Err(_) => {
                        Either::Right(
                            view! {
                                <p class="text-xs text-danger-text">
                                    {t_string!(i18n, generator.err_no_class).to_owned()}
                                </p>
                            },
                        )
                    }
                }}
            </div>

            // ---- Length --------------------------------------------------
            <div class="space-y-2">
                <div class="flex items-center justify-between">
                    <span class="text-sm font-medium text-text-primary">
                        {move || t!(i18n, generator.length)}
                    </span>
                    <span class="text-sm tabular-nums text-text-secondary">
                        {move || cfg.get().length}
                    </span>
                </div>
                <Slider
                    min=4.0
                    max=128.0
                    step=1.0
                    value=Signal::derive(move || f64::from(cfg.get().length))
                    aria_label=Signal::derive(move || t_string!(i18n, generator.length).to_owned())
                    on_change=Callback::new(move |v: f64| cfg.update(|c| c.length = v as u32))
                />
            </div>

            // ---- Character classes + options -----------------------------
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3">
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.lowercase).to_owned())
                    checked=Signal::derive(move || cfg.get().classes.lowercase)
                    on_change=Callback::new(move |v: bool| cfg.update(|c| c.classes.lowercase = v))
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.uppercase).to_owned())
                    checked=Signal::derive(move || cfg.get().classes.uppercase)
                    on_change=Callback::new(move |v: bool| cfg.update(|c| c.classes.uppercase = v))
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.digits).to_owned())
                    checked=Signal::derive(move || cfg.get().classes.digits)
                    on_change=Callback::new(move |v: bool| cfg.update(|c| c.classes.digits = v))
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.symbols).to_owned())
                    checked=Signal::derive(move || cfg.get().classes.symbols)
                    on_change=Callback::new(move |v: bool| cfg.update(|c| c.classes.symbols = v))
                />
                <OptionToggle
                    label=Signal::derive(move || {
                        t_string!(i18n, generator.exclude_ambiguous).to_owned()
                    })
                    checked=Signal::derive(move || cfg.get().exclude_ambiguous)
                    on_change=Callback::new(move |v: bool| cfg.update(|c| c.exclude_ambiguous = v))
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.require_each).to_owned())
                    checked=Signal::derive(move || cfg.get().require_each_selected)
                    on_change=Callback::new(move |v: bool| {
                        cfg.update(|c| c.require_each_selected = v);
                    })
                />
            </div>
        </div>
    }
}

/// A labelled on/off row for one character class or option.
#[component]
fn OptionToggle(
    #[prop(into)] label: Signal<String>,
    checked: Signal<bool>,
    on_change: Callback<bool>,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between gap-3">
            <span class="text-sm text-text-primary">{move || label.get()}</span>
            <Toggle checked=checked on_change=on_change aria_label=label />
        </div>
    }
}
