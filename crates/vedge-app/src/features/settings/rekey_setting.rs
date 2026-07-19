//! Re-key vault (slice 5.8) — the Credentials-tab danger zone + dialog.
//!
//! The heavy sibling of change-password: it mints a fresh DEK per entry and re-encrypts EVERY
//! surface, so a copied `.vdb` (and the DEKs an attacker extracted from it) can no longer open
//! anything going forward. Because it retires the local snapshot store and crosses a credential
//! boundary, on success the backend LOCKS the vault and we navigate to the launch screen so the
//! user re-unlocks against the re-keyed state (the 5.7 revert-locks precedent).
//!
//! Intent-labelled copy: *change password* = "different password"; *re-key* = "my file was
//! copied". Progress streams over a Tauri `Channel` into a determinate `ProgressBar` (a Spinner
//! covers the pre-first-tick moment) with a Cancel that aborts before the commit point.
//!
//! 🔴 A rotated Secret Key is show-once, but re-key LOCKS the vault, so it CANNOT be shown here —
//! the auto-lock watcher would eject the dialog within ~1 s. It is stashed in the `NewSecretKit`
//! context (above the router) and shown on the LAUNCH screen (no auto-lock there); we nav there
//! immediately on success, beating the auto-lock poll so no "session expired" toast fires.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::{Button, Input, PasswordStrengthMeter, ProgressBar, Spinner, Toggle};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

/// Percent complete `done/total` as a lint-clean `f64` (via `u32` so there's no
/// `cast_precision_loss`). `total == 0` → 0.0.
fn percent(done: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let d = f64::from(u32::try_from(done).unwrap_or(u32::MAX));
    let t = f64::from(u32::try_from(total).unwrap_or(u32::MAX));
    (d / t) * 100.0
}

use vedge_ipc::RekeyInputDto;

use crate::api;
use crate::api::error::ApiError;
use crate::features::vault::context::{ActiveVault, NewSecretKit};
use crate::features::vault::password_strength::score as password_score;
use crate::i18n::{t, t_string, use_i18n};

/// Minimum acceptable strength (0–4) for the new master password — same floor as change-password.
const MIN_STRENGTH: u8 = 2;

#[component]
pub fn RekeySetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    // Where a rotated Secret Key is handed off for the launch screen to show (re-key locks here).
    let new_kit = expect_context::<NewSecretKit>();
    let toast = use_toast();

    let dialog_open = RwSignal::new(false);
    let current = RwSignal::new(String::new());
    let next = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let rotate_sk = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    // `Some((done, total))` → live progress streamed from the backend over a Tauri Channel (a
    // determinate bar); `None` → not started yet (Spinner).
    let progress = RwSignal::new(Option::<(u64, u64)>::None);

    let vault_path = move || active.path.get().unwrap_or_default();
    let strength = Memo::new(move |_| password_score(&next.get()));
    let can_submit = move || {
        !current.get().is_empty()
            && !next.get().is_empty()
            && next.get() == confirm.get()
            && strength.get() >= MIN_STRENGTH
    };

    let open = Callback::new(move |()| {
        error.set(None);
        current.set(String::new());
        next.set(String::new());
        confirm.set(String::new());
        rotate_sk.set(false);
        dialog_open.set(true);
    });
    let close = Callback::new(move |()| dialog_open.set(false));

    let submit = move || {
        if busy.get() || !can_submit() {
            return;
        }
        error.set(None);
        busy.set(true);
        progress.set(None);
        let path = vault_path();
        let dto = RekeyInputDto {
            current_password: current.get(),
            new_password: next.get(),
            rotate_secret_key: rotate_sk.get(),
        };
        // Hoist localized strings + the navigator out of the async block (owner-safe).
        let msg_ok = t_string!(i18n, settings.rekey_success).to_owned();
        let msg_cancelled = t_string!(i18n, settings.rekey_cancelled).to_owned();
        let msg_wrong = t_string!(i18n, settings.change_password_wrong).to_owned();
        let msg_no_keychain = t_string!(i18n, settings.kit_reexport_no_keychain).to_owned();
        let msg_err = t_string!(i18n, settings.change_password_err).to_owned();
        let dismiss = t_string!(i18n, settings.dismiss).to_owned();
        let nav = use_navigate();

        // Progress streams over a Tauri Channel — this callback fires per entry as the backend
        // re-encrypts (a push, not a poll); `try_set` no-ops if the dialog has since unmounted.
        let on_progress = move |done: u64, total: u64| {
            progress.try_set(Some((done, total)));
        };

        spawn_local(async move {
            let result = api::rekey::rekey_vault(&path, &dto, on_progress).await;
            match result {
                Ok(result) if result.cancelled => {
                    toast.show(
                        ToastInput::new(msg_cancelled)
                            .variant(ToastVariant::Warning)
                            .dismiss_label(dismiss),
                    );
                    busy.set(false);
                    progress.set(None);
                    dialog_open.set(false);
                }
                Ok(result) => {
                    // Re-keyed + LOCKED. Hand a rotated Secret Key to the launch screen (it can't be
                    // shown here — the auto-lock watcher would eject this dialog within ~1 s), then
                    // navigate there IMMEDIATELY (beating that poll) to re-unlock against the
                    // re-keyed state. The launch screen shows the key with no auto-lock to cut it off.
                    if let Some(display) = result.secret_key_display {
                        new_kit.display.set(Some(display));
                    }
                    toast.show(
                        ToastInput::new(msg_ok)
                            .variant(ToastVariant::Success)
                            .dismiss_label(dismiss),
                    );
                    nav("/", Default::default());
                }
                Err(ApiError::WrongCredentials) => {
                    error.set(Some(msg_wrong));
                    busy.set(false);
                    progress.set(None);
                }
                Err(ApiError::Keychain(_)) => {
                    error.set(Some(msg_no_keychain));
                    busy.set(false);
                    progress.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("{msg_err} {e}")));
                    busy.set(false);
                    progress.set(None);
                }
            }
        });
    };

    // Abort a running re-key before its commit point (touches only the outer session lock, so it
    // reaches the re-key even though it holds the session guard).
    let cancel_op = move || {
        let path = vault_path();
        spawn_local(async move {
            let _ = api::rekey::cancel_rekey(&path).await;
        });
    };

    view! {
        <div class="flex items-start justify-between gap-5 py-3.5">
            <div>
                <div class="text-sm font-medium" style="color:var(--color-danger-text)">
                    {move || t!(i18n, settings.rekey_title)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.rekey_desc)}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                <Button
                    variant=Variant::Danger
                    attr:data-testid="rekey-vault"
                    on:click=move |_: web_sys::MouseEvent| open.run(())
                >
                    {move || t!(i18n, settings.rekey_button)}
                </Button>
            </div>
        </div>

        <Dialog
            open=Signal::derive(move || dialog_open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.rekey_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-4 min-w-[22rem]">
                    <Show
                        when=move || !busy.get()
                        fallback=move || {
                            view! {
                                <div class="flex flex-col items-center gap-3 py-4">
                                    <Show
                                        when=move || progress.get().is_some()
                                        fallback=|| view! { <Spinner /> }
                                    >
                                        <div class="w-full">
                                            <ProgressBar
                                                value=Signal::derive(move || {
                                                    progress.get().map_or(0.0, |(d, t)| percent(d, t))
                                                })
                                                show_value=true
                                                aria_label=Signal::derive(move || {
                                                    t_string!(i18n, settings.rekey_working).to_owned()
                                                })
                                            />
                                            <p class="text-xs text-text-secondary text-center mt-1">
                                                {move || {
                                                    progress.get().map(|(d, t)| format!("{d} / {t}"))
                                                }}
                                            </p>
                                        </div>
                                    </Show>
                                    <p class="text-sm text-text-secondary text-center">
                                        {move || t!(i18n, settings.rekey_working)}
                                    </p>
                                    <Button
                                        variant=Variant::Ghost
                                        size=Size::Sm
                                        attr:data-testid="rekey-cancel-op"
                                        on:click=move |_: web_sys::MouseEvent| cancel_op()
                                    >
                                        {move || t!(i18n, settings.rekey_cancel_op)}
                                    </Button>
                                </div>
                            }
                        }
                    >
                        // 🔴 The danger-zone warning: what re-key does, and what it cannot undo.
                        <div
                            class="rounded-lg border p-3 text-xs"
                            style="border-color:var(--color-danger);color:var(--color-danger-text)"
                        >
                            {move || t!(i18n, settings.rekey_warning)}
                        </div>

                        <FormField
                            id="rekey-current-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, settings.change_password_current).to_owned()
                            })
                        >
                            <Input
                                id="rekey-current-password"
                                input_type="password"
                                value=Signal::derive(move || current.get())
                                on_input=Callback::new(move |v: String| current.set(v))
                                reveal_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.show_password).to_owned()
                                })
                                hide_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.hide_password).to_owned()
                                })
                            />
                        </FormField>

                        <div class="flex flex-col gap-1.5">
                            <FormField
                                id="rekey-new-password"
                                label=Signal::derive(move || {
                                    t_string!(i18n, settings.change_password_new).to_owned()
                                })
                            >
                                <Input
                                    id="rekey-new-password"
                                    input_type="password"
                                    value=Signal::derive(move || next.get())
                                    on_input=Callback::new(move |v: String| next.set(v))
                                    reveal_label=Signal::derive(move || {
                                        t_string!(i18n, onboarding.show_password).to_owned()
                                    })
                                    hide_label=Signal::derive(move || {
                                        t_string!(i18n, onboarding.hide_password).to_owned()
                                    })
                                />
                            </FormField>
                            <PasswordStrengthMeter
                                score=strength
                                strength_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.strength_label).to_owned()
                                })
                                level_labels=Signal::derive(move || {
                                    [
                                        t_string!(i18n, onboarding.strength_weak).to_owned(),
                                        t_string!(i18n, onboarding.strength_fair).to_owned(),
                                        t_string!(i18n, onboarding.strength_good).to_owned(),
                                        t_string!(i18n, onboarding.strength_strong).to_owned(),
                                    ]
                                })
                            />
                        </div>

                        <FormField
                            id="rekey-confirm-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, settings.change_password_confirm).to_owned()
                            })
                        >
                            <Input
                                id="rekey-confirm-password"
                                input_type="password"
                                value=Signal::derive(move || confirm.get())
                                on_input=Callback::new(move |v: String| confirm.set(v))
                                reveal_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.show_password).to_owned()
                                })
                                hide_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.hide_password).to_owned()
                                })
                            />
                        </FormField>

                        <div class="flex items-start justify-between gap-4">
                            <div>
                                <div class="text-sm font-medium text-text-primary">
                                    {move || t!(i18n, settings.rekey_rotate_sk)}
                                </div>
                                <div class="text-xs text-text-secondary mt-0.5">
                                    {move || t!(i18n, settings.rekey_rotate_sk_desc)}
                                </div>
                            </div>
                            <div class="shrink-0 mt-0.5">
                                <Toggle
                                    checked=Signal::derive(move || rotate_sk.get())
                                    on_change=Callback::new(move |v: bool| rotate_sk.set(v))
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, settings.rekey_rotate_sk).to_owned()
                                    })
                                />
                            </div>
                        </div>

                        {move || {
                            error
                                .get()
                                .map(|e| {
                                    view! {
                                        <p class="text-sm" style="color:var(--color-danger-text)">
                                            {e}
                                        </p>
                                    }
                                })
                        }}

                        <div class="flex justify-end gap-2">
                            <Button
                                variant=Variant::Ghost
                                size=Size::Sm
                                on:click=move |_: web_sys::MouseEvent| close.run(())
                            >
                                {move || t!(i18n, settings.cancel)}
                            </Button>
                            {move || {
                                let disabled = !can_submit();
                                view! {
                                    <Button
                                        variant=Variant::Danger
                                        size=Size::Sm
                                        disabled=disabled
                                        attr:data-testid="rekey-submit"
                                        on:click=move |_: web_sys::MouseEvent| submit()
                                    >
                                        {move || t!(i18n, settings.rekey_confirm)}
                                    </Button>
                                }
                            }}
                        </div>
                    </Show>
                </div>
            </DialogBody>
        </Dialog>
    }
}
