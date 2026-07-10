//! Standalone password-generator panel (slice 3.1).
//!
//! Exercises the pure [`vedge_generator`] engine with **zero IPC** — generation
//! is synchronous charset math in the renderer. The config lives in one
//! `RwSignal<RandomConfig>`; the secret in a [`Zeroizing`] container (wiped on
//! drop). The entropy meter is a `Memo` over the config, so it updates live the
//! instant a class is toggled — even before Regenerate — and can never disagree
//! with the generator (both call the engine's one entropy code path).
//!
//! **Copy** routes through the hardened native `copy_text` command (slice 3.2)
//! via `CopyButton`'s `copy_with` strategy — so the copied secret carries the OS
//! no-history / no-cloud exclusion hints and is auto-cleared after the configured
//! delay (`SecurityPrefs.clipboard_clear_seconds`). The plain `String` that
//! reaches the DOM for display, and crosses IPC once for the copy, is outside
//! Rust's zeroize control — a documented residual, not a regression.
//!
//! Entropy bands come from `entropy_band` (charset math) — **not** from the
//! `password_strength` (zxcvbn) scorer, which is for *user-entered* passwords and
//! is kept strictly separate per the generator-entropy research note.

use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_generator::{RandomConfig, entropy_band, generate_random, random_entropy_bits};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{
    Button, CopyButton, CopyFuture, IconButton, PasswordStrengthMeter, Slider, Toggle,
};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};
use zeroize::Zeroizing;

use crate::api;
use crate::features::settings::generator_prefs::{self, GeneratorPrefsCtx};
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn GeneratorPanel(
    /// When present, renders a "Use this password" button that emits the current
    /// secret (used by the create/edit-form popover). When `None` (the standalone
    /// `/v/generator` page) the panel keeps only reveal / copy / regenerate.
    #[prop(into, default = None)]
    on_use: Option<Callback<String>>,
) -> impl IntoView {
    let i18n = use_i18n();

    // Shared preset (last-used config): the inline quick-generate button, this
    // panel, and the standalone page all read/write one `GeneratorPrefsCtx`.
    let prefs = expect_context::<GeneratorPrefsCtx>().0;
    let reveal = RwSignal::new(true);
    // Seed with a first secret at mount (any valid preset is fine).
    let secret = RwSignal::new(
        generate_random(&RandomConfig::from(prefs.get_untracked()))
            .map(|g| g.secret)
            .unwrap_or_default(),
    );

    // Copy routes through the hardened native `copy_text` command (slice 3.2).
    // Toast helpers read the dismiss label `untrack`ed so they're safe to call
    // from inside `spawn_local`.
    let sec = expect_context::<SecurityPrefsCtx>();
    let toast = use_toast();
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, generator.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, generator.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };
    // Route the copy through the hardened native command. The returned bool
    // drives `CopyButton`'s check-mark; the toast gives the explicit result.
    let copy_native = Callback::new(move |v: String| -> CopyFuture {
        // Read locale strings + prefs in the handler body (reactive owner
        // present); reading them inside the future would trip the "outside a
        // reactive tracking context" warning.
        let secs = sec.0.get_untracked().clipboard_clear_seconds;
        let ok_msg = t_string!(i18n, generator.copied).to_owned();
        let err_msg = t_string!(i18n, generator.copy_failed).to_owned();
        Box::pin(async move {
            if api::clipboard::copy_text(&v, Some(secs)).await.is_ok() {
                show_success(ok_msg);
                true
            } else {
                show_error(err_msg);
                false
            }
        })
    });

    // Synchronous — pure math, no `spawn_local`. Reads the preset untracked so the
    // handler doesn't subscribe to it.
    let regenerate = move |()| match generate_random(&RandomConfig::from(prefs.get_untracked())) {
        Ok(g) => secret.set(g.secret),
        Err(_) => secret.set(Zeroizing::default()),
    };

    // Live entropy of the *process*. Gates the meter; the secret only changes on
    // Regenerate, but the meter tracks every preset edit.
    let entropy = Memo::new(move |_| random_entropy_bits(&RandomConfig::from(prefs.get())));
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
                    copy_with=copy_native
                    label=Signal::derive(move || t_string!(i18n, generator.copy).to_owned())
                    copied_label=Signal::derive(move || {
                        t_string!(i18n, generator.copied).to_owned()
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
                        {move || prefs.get().length}
                    </span>
                </div>
                <Slider
                    min=4.0
                    max=128.0
                    step=1.0
                    value=Signal::derive(move || f64::from(prefs.get().length))
                    aria_label=Signal::derive(move || t_string!(i18n, generator.length).to_owned())
                    // Live update every drag tick (feeds the meter); persist once
                    // on release so a drag isn't ~120 fire-and-forget KV writes.
                    on_change=Callback::new(move |v: f64| prefs.update(|p| p.length = v as u32))
                    on_change_end=Callback::new(move |v: f64| {
                        prefs.update(|p| p.length = v as u32);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
            </div>

            // ---- Character classes + options -----------------------------
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3">
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.lowercase).to_owned())
                    checked=Signal::derive(move || prefs.get().lowercase)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.lowercase = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.uppercase).to_owned())
                    checked=Signal::derive(move || prefs.get().uppercase)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.uppercase = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.digits).to_owned())
                    checked=Signal::derive(move || prefs.get().digits)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.digits = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.symbols).to_owned())
                    checked=Signal::derive(move || prefs.get().symbols)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.symbols = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || {
                        t_string!(i18n, generator.exclude_ambiguous).to_owned()
                    })
                    checked=Signal::derive(move || prefs.get().exclude_ambiguous)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.exclude_ambiguous = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.require_each).to_owned())
                    checked=Signal::derive(move || prefs.get().require_each_selected)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.require_each_selected = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
            </div>

            // ---- Use this password (form popover only) -------------------
            {on_use
                .map(|cb| {
                    view! {
                        <Button
                            variant=Variant::Primary
                            full_width=true
                            on:click=move |_| cb.run(secret.with(|z| z.as_str().to_owned()))
                        >
                            {move || t_string!(i18n, generator.use_password).to_owned()}
                        </Button>
                    }
                })}
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
