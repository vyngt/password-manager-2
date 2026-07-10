//! Standalone password-generator panel (slices 3.1 + 3.4).
//!
//! Exercises the pure [`vedge_generator`] engine with **zero IPC** — generation
//! is synchronous math in the renderer. A **mode picker** ([`SegmentedControl`])
//! selects random / passphrase / PIN / pronounceable / pattern; the per-mode
//! config controls swap beneath it, while the **Output row and entropy meter
//! stay shared** — both read the same active [`GenSpec`], so the meter can never
//! disagree with the generator across any mode.
//!
//! The persisted preset ([`GeneratorPrefsCtx`]) holds the selected mode plus each
//! mode's `Copy` config; the free-text **pattern** string is a session-local
//! signal (not persisted, so `GeneratorPrefs` stays `Copy`).
//!
//! **Copy** routes through the hardened native `copy_text` command (slice 3.2)
//! via `CopyButton`'s `copy_with` strategy — so the copied secret carries the OS
//! no-history / no-cloud exclusion hints and is auto-cleared after the configured
//! delay. The plain `String` that reaches the DOM for display, and crosses IPC
//! once for the copy, is outside Rust's zeroize control — a documented residual.
//!
//! Entropy bands come from `entropy_band` (process-entropy math) — **not** from
//! the `password_strength` (zxcvbn) scorer, which is for *user-entered* passwords
//! and is kept strictly separate per the generator-entropy research note.

use icondata as i;
use leptos::either::{Either, EitherOf5};
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_generator::{
    GenError, GenSpec, PatternConfig, RandomConfig, entropy_band, entropy_bits, generate,
};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{
    Button, CopyButton, CopyFuture, IconButton, Input, PasswordStrengthMeter, SegmentOption,
    SegmentedControl, Select, SelectItem, Slider, Toggle,
};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};
use zeroize::Zeroizing;

use crate::api;
use crate::features::settings::generator_prefs::{
    self, GenMode, GeneratorPrefs, GeneratorPrefsCtx, SeparatorPref,
};
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::i18n::{t, t_string, use_i18n};

/// Build the active [`GenSpec`] from the persisted preset + the session pattern.
fn build_spec(prefs: GeneratorPrefs, pattern: String) -> GenSpec {
    match prefs.mode {
        GenMode::Random => GenSpec::Random(RandomConfig::from(prefs)),
        GenMode::Passphrase => GenSpec::Passphrase(prefs.passphrase_config()),
        GenMode::Pin => GenSpec::Pin(prefs.pin_config()),
        GenMode::Pronounceable => GenSpec::Pronounceable(prefs.pron_config()),
        GenMode::Pattern => GenSpec::Pattern(PatternConfig { pattern }),
    }
}

#[component]
pub fn GeneratorPanel(
    /// When present, renders a "Use this password" button that emits the current
    /// secret (used by the create/edit-form popover). When `None` (the standalone
    /// `/v/generator` page) the panel keeps only reveal / copy / regenerate.
    #[prop(into, default = None)]
    on_use: Option<Callback<String>>,
) -> impl IntoView {
    let i18n = use_i18n();

    // Shared preset (last-used mode + per-mode config).
    let prefs = expect_context::<GeneratorPrefsCtx>().0;
    // Free-text pattern is session-local (keeps GeneratorPrefs `Copy`).
    let pattern_text = RwSignal::new(PatternConfig::default().pattern);
    let reveal = RwSignal::new(true);

    // Seed with a first secret at mount (untracked — component body).
    let secret = RwSignal::new(
        generate(&build_spec(
            prefs.get_untracked(),
            pattern_text.get_untracked(),
        ))
        .map(|g| g.secret)
        .unwrap_or_default(),
    );

    // Copy routes through the hardened native `copy_text` command (slice 3.2).
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
    let copy_native = Callback::new(move |v: String| -> CopyFuture {
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
    // handler doesn't subscribe. Copy closure, so both the button and the mode
    // picker's `on_change` can invoke it.
    let regenerate = move |()| match generate(&build_spec(
        prefs.get_untracked(),
        pattern_text.get_untracked(),
    )) {
        Ok(g) => secret.set(g.secret),
        Err(_) => secret.set(Zeroizing::default()),
    };

    // Live entropy of the active *process*. Tracks every preset/pattern edit; the
    // secret itself only changes on Regenerate or a mode switch.
    let entropy = Memo::new(move |_| entropy_bits(&build_spec(prefs.get(), pattern_text.get())));
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

    // Mode picker: only re-derives the swapped section when the *mode* changes.
    let mode = Memo::new(move |_| prefs.get().mode);
    let on_mode_change = Callback::new(move |v: String| {
        prefs.update(|p| p.mode = GenMode::from_value(&v));
        generator_prefs::save(&prefs.get_untracked());
        regenerate(());
    });
    // Mode-picker labels are **reactive** — `Signal::derive` defers the `t_string!`
    // read to render time (no eager read in the body → no `untrack`), so the
    // picker relocalizes on live language switch. `SegmentOption::text` accepts a
    // `Signal<String>` via its `TextProp` label.
    let mode_options: Vec<SegmentOption> = GenMode::ALL
        .into_iter()
        .map(|m| {
            let label = Signal::derive(move || match m {
                GenMode::Random => t_string!(i18n, generator.mode_random).to_owned(),
                GenMode::Passphrase => t_string!(i18n, generator.mode_passphrase).to_owned(),
                GenMode::Pin => t_string!(i18n, generator.mode_pin).to_owned(),
                GenMode::Pronounceable => t_string!(i18n, generator.mode_pronounceable).to_owned(),
                GenMode::Pattern => t_string!(i18n, generator.mode_pattern).to_owned(),
            });
            SegmentOption::text(m.as_value(), label)
        })
        .collect();
    // Separator options are built once (untracked) — the `Select` component's
    // option list is non-reactive, so these don't relocalize on live language
    // switch. Stored so the swapped passphrase arm can re-read them per mode switch.
    let separator_options = StoredValue::new(untrack(|| {
        SeparatorPref::ALL
            .into_iter()
            .map(|s| {
                let label = match s {
                    SeparatorPref::Hyphen => t_string!(i18n, generator.sep_hyphen),
                    SeparatorPref::Space => t_string!(i18n, generator.sep_space),
                    SeparatorPref::Dot => t_string!(i18n, generator.sep_dot),
                    SeparatorPref::Underscore => t_string!(i18n, generator.sep_underscore),
                    SeparatorPref::None => t_string!(i18n, generator.sep_none),
                };
                SelectItem::option(s.as_value(), label.to_owned())
            })
            .collect::<Vec<_>>()
    }));

    view! {
        <div class="space-y-6">
            // ---- Mode picker --------------------------------------------
            <SegmentedControl
                options=mode_options
                value=Signal::derive(move || mode.get().as_value().to_owned())
                on_change=on_mode_change
                aria_label=Signal::derive(move || t_string!(i18n, generator.mode).to_owned())
            />

            // ---- Output (shared) ----------------------------------------
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

            // ---- Entropy meter (shared) ---------------------------------
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
                    Err(e) => {
                        let msg = move || match e {
                            GenError::NoClassSelected | GenError::EmptyEffectiveAlphabet => {
                                t_string!(i18n, generator.err_no_class).to_owned()
                            }
                            GenError::LengthOutOfRange => {
                                t_string!(i18n, generator.err_length).to_owned()
                            }
                            GenError::WordCountOutOfRange => {
                                t_string!(i18n, generator.err_word_count).to_owned()
                            }
                            GenError::EmptyPattern => {
                                t_string!(i18n, generator.err_empty_pattern).to_owned()
                            }
                            GenError::InvalidPattern => {
                                t_string!(i18n, generator.err_invalid_pattern).to_owned()
                            }
                        };
                        Either::Right(view! { <p class="text-xs text-danger-text">{msg}</p> })
                    }
                }}
            </div>

            // ---- Per-mode config (swaps on mode change) -----------------
            {move || match mode.get() {
                GenMode::Random => EitherOf5::A(view! { <RandomControls prefs=prefs /> }),
                GenMode::Passphrase => {
                    EitherOf5::B(
                        view! {
                            <PassphraseControls prefs=prefs separator_options=separator_options />
                        },
                    )
                }
                GenMode::Pin => EitherOf5::C(view! { <PinControls prefs=prefs /> }),
                GenMode::Pronounceable => {
                    EitherOf5::D(view! { <PronounceableControls prefs=prefs /> })
                }
                GenMode::Pattern => {
                    EitherOf5::E(view! { <PatternControls pattern_text=pattern_text /> })
                }
            }}

            // ---- Use this password (form popover only) ------------------
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

/// A labelled slider row (header with live value + the slider). Persists on
/// release so a drag isn't ~120 fire-and-forget KV writes.
#[component]
fn LabelledSlider(
    #[prop(into)] label: Signal<String>,
    value: Signal<u32>,
    min: f64,
    max: f64,
    on_input: Callback<u32>,
    on_commit: Callback<u32>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <div class="flex items-center justify-between">
                <span class="text-sm font-medium text-text-primary">{move || label.get()}</span>
                <span class="text-sm tabular-nums text-text-secondary">{move || value.get()}</span>
            </div>
            <Slider
                min=min
                max=max
                step=1.0
                value=Signal::derive(move || f64::from(value.get()))
                aria_label=label
                on_change=Callback::new(move |v: f64| on_input.run(v as u32))
                on_change_end=Callback::new(move |v: f64| on_commit.run(v as u32))
            />
        </div>
    }
}

/// Random-mode config: length + character-class grid.
#[component]
fn RandomControls(prefs: RwSignal<GeneratorPrefs>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-6">
            <LabelledSlider
                label=Signal::derive(move || t_string!(i18n, generator.length).to_owned())
                value=Signal::derive(move || prefs.get().length)
                min=4.0
                max=128.0
                on_input=Callback::new(move |v: u32| prefs.update(|p| p.length = v))
                on_commit=Callback::new(move |v: u32| {
                    prefs.update(|p| p.length = v);
                    generator_prefs::save(&prefs.get_untracked());
                })
            />
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
        </div>
    }
}

/// Passphrase-mode config: word count + separator + capitalize/number.
#[component]
fn PassphraseControls(
    prefs: RwSignal<GeneratorPrefs>,
    separator_options: StoredValue<Vec<SelectItem>>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-6">
            <LabelledSlider
                label=Signal::derive(move || t_string!(i18n, generator.words).to_owned())
                value=Signal::derive(move || prefs.get().words)
                min=3.0
                max=12.0
                on_input=Callback::new(move |v: u32| prefs.update(|p| p.words = v))
                on_commit=Callback::new(move |v: u32| {
                    prefs.update(|p| p.words = v);
                    generator_prefs::save(&prefs.get_untracked());
                })
            />
            <div class="space-y-2">
                <span class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, generator.separator)}
                </span>
                <Select
                    options=separator_options.get_value()
                    value=Signal::derive(move || prefs.get().separator.as_value().to_owned())
                    aria_label=Signal::derive(move || {
                        t_string!(i18n, generator.separator).to_owned()
                    })
                    on_change=Callback::new(move |v: String| {
                        prefs.update(|p| p.separator = SeparatorPref::from_value(&v));
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3">
                <OptionToggle
                    label=Signal::derive(move || t_string!(i18n, generator.capitalize).to_owned())
                    checked=Signal::derive(move || prefs.get().capitalize)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.capitalize = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
                <OptionToggle
                    label=Signal::derive(move || {
                        t_string!(i18n, generator.include_number).to_owned()
                    })
                    checked=Signal::derive(move || prefs.get().include_number)
                    on_change=Callback::new(move |v: bool| {
                        prefs.update(|p| p.include_number = v);
                        generator_prefs::save(&prefs.get_untracked());
                    })
                />
            </div>
        </div>
    }
}

/// PIN-mode config: length + the honest context caption.
#[component]
fn PinControls(prefs: RwSignal<GeneratorPrefs>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-2">
            <LabelledSlider
                label=Signal::derive(move || t_string!(i18n, generator.pin_length).to_owned())
                value=Signal::derive(move || prefs.get().pin_length)
                min=4.0
                max=12.0
                on_input=Callback::new(move |v: u32| prefs.update(|p| p.pin_length = v))
                on_commit=Callback::new(move |v: u32| {
                    prefs.update(|p| p.pin_length = v);
                    generator_prefs::save(&prefs.get_untracked());
                })
            />
            <p class="text-xs text-text-secondary">{move || t!(i18n, generator.pin_hint)}</p>
        </div>
    }
}

/// Pronounceable-mode config: length only.
#[component]
fn PronounceableControls(prefs: RwSignal<GeneratorPrefs>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <LabelledSlider
            label=Signal::derive(move || t_string!(i18n, generator.length).to_owned())
            value=Signal::derive(move || prefs.get().pron_length)
            min=6.0
            max=32.0
            on_input=Callback::new(move |v: u32| prefs.update(|p| p.pron_length = v))
            on_commit=Callback::new(move |v: u32| {
                prefs.update(|p| p.pron_length = v);
                generator_prefs::save(&prefs.get_untracked());
            })
        />
    }
}

/// Pattern-mode config: the token grammar input + a legend.
#[component]
fn PatternControls(pattern_text: RwSignal<String>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-2">
            <span class="text-sm font-medium text-text-primary">
                {move || t!(i18n, generator.pattern)}
            </span>
            <Input
                id="gen-pattern"
                value=Signal::derive(move || pattern_text.get())
                on_input=Callback::new(move |v: String| pattern_text.set(v))
                aria_label=Signal::derive(move || t_string!(i18n, generator.pattern).to_owned())
            />
            <p class="text-xs text-text-secondary">{move || t!(i18n, generator.pattern_legend)}</p>
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
