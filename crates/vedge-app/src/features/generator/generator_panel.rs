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
//! the `password_strength` heuristic scorer (a length + char-class estimate, *not*
//! zxcvbn — the crate is not a dependency), which is for *user-entered* passwords
//! and is kept strictly separate per the generator-entropy research note.

use icondata as i;
use leptos::either::{Either, EitherOf5};
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_generator::{
    GenError, GenSpec, MAX_BATCH, PatternConfig, RandomConfig, entropy_band, entropy_bits,
    generate, generate_many,
};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{
    Button, CopyButton, CopyFuture, EmptyState, IconButton, Input, Label, NumberInput,
    PasswordStrengthMeter, SegmentOption, SegmentedControl, Select, SelectItem, Slider, Toggle,
};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};
use zeroize::Zeroizing;

use crate::api;
use crate::features::generator::history::{GeneratedHistoryCtx, HistoryItem};
use crate::features::settings::generator_prefs::{
    self, GenMode, GeneratorPrefs, GeneratorPrefsCtx, SeparatorPref,
};
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::i18n::{t, t_string, use_i18n};

/// One row of a bulk-generate batch. Session-local (dropped on Regenerate / next
/// batch / mode switch); its `Zeroizing` secret wipes on drop. The batch is always
/// replaced wholesale, so the list is rendered by a plain map (no `<For>` keying —
/// index keys would let a same-length new batch reuse stale row DOM).
#[derive(Clone)]
struct BulkRow {
    secret: Zeroizing<String>,
    entropy_bits: f64,
}

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
    // App-root session history (Zeroizing, capped, wiped on lock). Fed by
    // Regenerate + bulk pick/copy below; provided once in `app.rs`.
    let history = expect_context::<GeneratedHistoryCtx>();
    // Free-text pattern is session-local (keeps GeneratorPrefs `Copy`).
    let pattern_text = RwSignal::new(PatternConfig::default().pattern);
    let reveal = RwSignal::new(true);

    // Bulk-generate state — all session-local. `bulk_count` is the requested N;
    // `bulk` is the current batch (replaced wholesale, dropped→zeroized on the
    // next batch / Regenerate / mode switch). The two sections default open on the
    // standalone page and collapsed in the form popover (`on_use.is_some()`) so the
    // popover stays compact until the user opts in.
    let bulk_count = RwSignal::new(10u32);
    let bulk = RwSignal::new(Vec::<BulkRow>::new());
    let bulk_open = RwSignal::new(on_use.is_none());
    let recent_open = RwSignal::new(on_use.is_none());

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

    // Map any engine error to a localized message (event-handler / reactive
    // context, so the `t_string!` reads have an owner). Shared by the bulk toast
    // and the entropy-meter error line; the `BatchOutOfRange` arm keeps the match
    // exhaustive even though `entropy_bits` never returns it.
    let gen_err_msg = move |e: GenError| -> String {
        match e {
            GenError::NoClassSelected | GenError::EmptyEffectiveAlphabet => {
                t_string!(i18n, generator.err_no_class).to_owned()
            }
            GenError::LengthOutOfRange => t_string!(i18n, generator.err_length).to_owned(),
            GenError::WordCountOutOfRange => t_string!(i18n, generator.err_word_count).to_owned(),
            GenError::EmptyPattern => t_string!(i18n, generator.err_empty_pattern).to_owned(),
            GenError::InvalidPattern => t_string!(i18n, generator.err_invalid_pattern).to_owned(),
            GenError::BatchOutOfRange => t_string!(i18n, generator.err_batch).to_owned(),
        }
    };

    // Synchronous — pure math, no `spawn_local`. Reads the preset untracked so the
    // handler doesn't subscribe. Copy closure, so both the button and the mode
    // picker's `on_change` can invoke it. Each generation feeds the session history
    // and drops the current bulk batch (the spec's "history fed by Regenerate" +
    // "bulk cleared on regenerate").
    let regenerate = move |()| match generate(&build_spec(
        prefs.get_untracked(),
        pattern_text.get_untracked(),
    )) {
        Ok(g) => {
            history.push(g.secret.clone(), g.entropy_bits);
            secret.set(g.secret);
            bulk.set(Vec::new());
        }
        Err(_) => secret.set(Zeroizing::default()),
    };

    // Bulk generate: N independent secrets from the active spec, replacing the
    // batch. Pure/sync (≤50 short strings), so no spinner. Errors → a Danger toast.
    let generate_batch = move |()| {
        let count = bulk_count.get_untracked();
        match generate_many(
            &build_spec(prefs.get_untracked(), pattern_text.get_untracked()),
            count,
        ) {
            Ok(items) => {
                let rows = items
                    .into_iter()
                    .map(|g| BulkRow {
                        secret: g.secret,
                        entropy_bits: g.entropy_bits,
                    })
                    .collect::<Vec<_>>();
                bulk.set(rows);
            }
            Err(e) => {
                bulk.set(Vec::new());
                show_error(gen_err_msg(e));
            }
        }
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
    // Separator options as a reactive `Signal` — the `Select` component now reads
    // its option list reactively, so the labels relocalize on a live language switch
    // (and the swapped passphrase arm re-reads them per mode switch).
    let separator_options = Signal::derive(move || {
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
    });

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
                    attr:data-testid="gen-regenerate"
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
                        let msg = move || gen_err_msg(e);
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

            // ---- Bulk generate (collapsible) ----------------------------
            <div class="space-y-3">
                // Whole-row disclosure toggle — a clickable row (§7-sanctioned
                // exception: whole clickable rows/cards, not a `Button`).
                <button
                    type="button"
                    class="flex w-full items-center justify-between rounded-md py-1 text-left text-sm font-semibold text-text-primary"
                    data-testid="gen-bulk-toggle"
                    aria-expanded=move || bulk_open.get().to_string()
                    on:click=move |_| bulk_open.update(|o| *o = !*o)
                >
                    <span>{move || t!(i18n, generator.bulk)}</span>
                    <span
                        class="text-text-secondary transition-transform"
                        class:rotate-90=move || bulk_open.get()
                        aria-hidden="true"
                    >
                        <Icon icon=i::FaChevronRightSolid />
                    </span>
                </button>
                <Show when=move || bulk_open.get()>
                    <div class="space-y-3">
                        <div class="flex items-end gap-3">
                            <div class="space-y-1.5">
                                <Label html_for="gen-bulk-count">
                                    {move || t!(i18n, generator.bulk_count)}
                                </Label>
                                <NumberInput
                                    id="gen-bulk-count"
                                    class="w-32"
                                    value=Signal::derive(move || f64::from(bulk_count.get()))
                                    min=Some(1.0)
                                    max=Some(f64::from(MAX_BATCH))
                                    step=1.0
                                    on_change=Callback::new(move |v: f64| {
                                        bulk_count.set(v.clamp(1.0, f64::from(MAX_BATCH)) as u32);
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, generator.bulk_count).to_owned()
                                    })
                                    decrement_label=Signal::derive(move || {
                                        t_string!(i18n, generator.bulk_count_dec).to_owned()
                                    })
                                    increment_label=Signal::derive(move || {
                                        t_string!(i18n, generator.bulk_count_inc).to_owned()
                                    })
                                />
                            </div>
                            <Button
                                variant=Variant::Secondary
                                attr:data-testid="gen-bulk-generate"
                                on:click=move |_| generate_batch(())
                            >
                                {move || t!(i18n, generator.bulk_generate)}
                            </Button>
                        </div>
                        <div class="space-y-2 max-h-56 overflow-y-auto">
                            {move || {
                                bulk.get()
                                    .into_iter()
                                    .map(|r: BulkRow| {
                                        let plain = r.secret.as_str().to_owned();
                                        let bits = r.entropy_bits;
                                        let p_copy = plain.clone();
                                        let use_cb = on_use
                                            .map(move |cb| {
                                                Callback::new(move |s: String| {
                                                    history.push(Zeroizing::new(s.clone()), bits);
                                                    cb.run(s);
                                                })
                                            });
                                        let copied_cb = Callback::new(move |()| {
                                            history.push(Zeroizing::new(p_copy.clone()), bits);
                                        });
                                        view! {
                                            <SecretRow
                                                secret=plain
                                                entropy_bits=bits
                                                copy_native=copy_native
                                                on_use=use_cb
                                                on_copied=Some(copied_cb)
                                            />
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </div>
                </Show>
            </div>

            // ---- Recent history (collapsible) ---------------------------
            <div class="space-y-3">
                <button
                    type="button"
                    class="flex w-full items-center justify-between rounded-md py-1 text-left text-sm font-semibold text-text-primary"
                    data-testid="gen-history-toggle"
                    aria-expanded=move || recent_open.get().to_string()
                    on:click=move |_| recent_open.update(|o| *o = !*o)
                >
                    <span>
                        {move || t!(i18n, generator.history)}
                        {move || {
                            let n = history.items.with(Vec::len);
                            if n > 0 { format!(" ({n})") } else { String::new() }
                        }}
                    </span>
                    <span
                        class="text-text-secondary transition-transform"
                        class:rotate-90=move || recent_open.get()
                        aria-hidden="true"
                    >
                        <Icon icon=i::FaChevronRightSolid />
                    </span>
                </button>
                <Show when=move || recent_open.get()>
                    <Show
                        when=move || history.items.with(|v| !v.is_empty())
                        fallback=move || {
                            view! {
                                <EmptyState
                                    icon=i::FaClockRotateLeftSolid
                                    title=Signal::derive(move || {
                                        t_string!(i18n, generator.history_empty).to_owned()
                                    })
                                    description=Signal::derive(move || {
                                        t_string!(i18n, generator.history_lock_note).to_owned()
                                    })
                                />
                            }
                        }
                    >
                        <div class="space-y-2">
                            <div class="space-y-2 max-h-56 overflow-y-auto">
                                <For
                                    each=move || history.items.get()
                                    key=|it| it.id
                                    children=move |it: HistoryItem| {
                                        let plain = it.secret.as_str().to_owned();
                                        let bits = it.entropy_bits;
                                        view! {
                                            <SecretRow
                                                attr:data-testid="gen-history-row"
                                                secret=plain
                                                entropy_bits=bits
                                                copy_native=copy_native
                                                on_use=on_use
                                            />
                                        }
                                    }
                                />
                            </div>
                            <div class="flex items-center justify-between gap-2 pt-1">
                                <span class="text-xs text-text-secondary">
                                    {move || t!(i18n, generator.history_lock_note)}
                                </span>
                                <Button
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    attr:data-testid="gen-history-clear"
                                    on:click=move |_| history.clear()
                                >
                                    {move || t!(i18n, generator.history_clear)}
                                </Button>
                            </div>
                        </div>
                    </Show>
                </Show>
            </div>
        </div>
    }
}

/// One masked secret row (bulk result or history entry): reveal + process
/// entropy + copy + optional pick. The plaintext reaches the DOM as a bare
/// `String` — the same documented residual as the primary Output row (the
/// session buffers themselves stay `Zeroizing`).
#[component]
fn SecretRow(
    secret: String,
    entropy_bits: f64,
    copy_native: Callback<String, CopyFuture>,
    /// "Use/pick" this secret (fills the form) — shown only when present.
    #[prop(into, default = None)]
    on_use: Option<Callback<String>>,
    /// Fired after a successful copy — bulk rows push into history; history rows
    /// omit it (they're already there).
    #[prop(into, default = None)]
    on_copied: Option<Callback<()>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let reveal = RwSignal::new(false);
    let char_count = secret.chars().count();
    let copy_val = secret.clone();
    let use_val = secret.clone();
    let display = {
        let s = secret;
        move || {
            if reveal.get() {
                s.clone()
            } else {
                "•".repeat(char_count)
            }
        }
    };
    // Compact, glanceable process-entropy band (reuses the meter's band labels).
    let band = entropy_band(entropy_bits);
    let band_label = Signal::derive(move || {
        match band {
            1 => t_string!(i18n, generator.band_weak),
            2 => t_string!(i18n, generator.band_fair),
            3 => t_string!(i18n, generator.band_strong),
            _ => t_string!(i18n, generator.band_excellent),
        }
        .to_owned()
    });
    let band_cls = match band {
        1 => "text-danger-text",
        2 => "text-warning-text",
        3 => "text-text-secondary",
        _ => "text-success-text",
    };
    let on_copy = on_copied.map(|cb| {
        Callback::new(move |ok: bool| {
            if ok {
                cb.run(());
            }
        })
    });
    view! {
        <div class="flex items-center gap-2 rounded-md border border-border p-2">
            <code class="flex-1 select-all break-all font-mono text-xs text-text-primary min-h-[1rem]">
                {display}
            </code>
            <span class=format!(
                "shrink-0 whitespace-nowrap text-xs font-medium {band_cls}",
            )>{move || band_label.get()}</span>
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
                value=Signal::derive(move || copy_val.clone())
                copy_with=copy_native
                label=Signal::derive(move || t_string!(i18n, generator.copy).to_owned())
                copied_label=Signal::derive(move || t_string!(i18n, generator.copied).to_owned())
                on_copy=on_copy
            />
            {on_use
                .map(|cb| {
                    let v = use_val.clone();
                    view! {
                        <IconButton
                            variant=Variant::Ghost
                            size=Size::Sm
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, generator.bulk_use).to_owned()
                            })
                            on_click=Callback::new(move |()| cb.run(v.clone()))
                        >
                            <span aria-hidden="true">
                                <Icon icon=i::FaCheckSolid />
                            </span>
                        </IconButton>
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
    separator_options: Signal<Vec<SelectItem>>,
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
                    options=separator_options
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
