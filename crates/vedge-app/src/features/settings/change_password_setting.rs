//! Change-master-password (slice 5.6 ②) — a Settings ▸ Security row + dialog.
//!
//! Wires the Phase-3 `change_password` command (finally consumed) behind a
//! current-password re-prompt. The command re-authenticates the current
//! password against the live session **before** the O(n) DEK rewrap (slice 5.6
//! ④), so an unlocked-but-unattended vault can't have its password changed by
//! whoever walks up. The rewrap keeps the session live, so on success we just
//! toast and close. `busy` gates the dialog against a double-submit — two
//! overlapping rewraps are undefined.

use chrono::Utc;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ui::components::feedback::dialog::{
    Dialog, DialogBody, DialogFooter, DialogHeader, DialogTitle,
};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::{Badge, Button, Input, PasswordStrengthMeter};
use vedge_ui::primitives::tokens::{
    BadgeSize, BadgeVariant, DialogSize, Size, ToastVariant, Variant,
};

use vedge_ipc::ChangePasswordInputDto;

use crate::api;
use crate::api::error::ApiError;
use crate::features::help_popover::HelpPopover;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::password_strength::score as password_score;
use crate::features::vault::timestamps::is_older_than_days;
use crate::i18n::{t, t_string, use_i18n};

/// Minimum acceptable strength (0–4) for the new master password — mirrors the
/// onboarding gate so a change can't weaken below what creation required.
const MIN_STRENGTH: u8 = 2;

/// Nudge the user to change the master password after this many days (slice 5.9 ③).
/// Conservative — a nagging password manager trains users to click through warnings.
const PASSWORD_STALE_DAYS: i64 = 365;

#[component]
pub fn ChangeMasterPasswordSetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let dialog_open = RwSignal::new(false);
    let current = RwSignal::new(String::new());
    let next = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    // Credential-age nudge (slice 5.9 ③): true when the master password is stale. Loaded on
    // mount (and on vault switch) via `credential_status`, the biometric_setting.rs pattern.
    let stale = RwSignal::new(false);
    Effect::new(move |_| {
        let path = active.path.get().unwrap_or_default();
        if path.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(status) = api::password::credential_status(&path).await {
                let now_ms = Utc::now().timestamp_millis();
                stale.set(is_older_than_days(
                    &status.password_changed_at,
                    now_ms,
                    PASSWORD_STALE_DAYS,
                ));
            }
        });
    });

    // Settings always runs inside an unlocked vault; the path drives the command.
    let vault_path = move || active.path.get().unwrap_or_default();

    let strength = Memo::new(move |_| password_score(&next.get()));
    // Submittable iff: current filled, the new password clears the strength
    // floor, and confirm matches. (The *current* password is verified
    // server-side; the client only guards the new one.)
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
        dialog_open.set(true);
    });
    let close = Callback::new(move |()| dialog_open.set(false));

    let submit = move || {
        if busy.get() || !can_submit() {
            return;
        }
        error.set(None);
        busy.set(true);
        let path = vault_path();
        let dto = ChangePasswordInputDto {
            current_password: current.get(),
            new_password: next.get(),
            new_secret_key_b64: None,
        };
        // Hoist localized strings out of the async block (owner-safe).
        let msg_ok = t_string!(i18n, settings.change_password_saved).to_owned();
        let msg_wrong = t_string!(i18n, settings.change_password_wrong).to_owned();
        let msg_err = t_string!(i18n, settings.change_password_err).to_owned();
        let msg_no_keychain = t_string!(i18n, settings.kit_reexport_no_keychain).to_owned();
        let dismiss = t_string!(i18n, settings.dismiss).to_owned();
        spawn_local(async move {
            match api::password::change_password(&path, &dto).await {
                Ok(()) => {
                    toast.show(
                        ToastInput::new(msg_ok)
                            .variant(ToastVariant::Success)
                            .dismiss_label(dismiss),
                    );
                    current.set(String::new());
                    next.set(String::new());
                    confirm.set(String::new());
                    dialog_open.set(false);
                    // Just changed → no longer stale (avoids a lingering nudge until remount).
                    stale.set(false);
                }
                // The command re-verifies the current password first; a mismatch
                // is `WrongCredentials` and nothing was rewrapped.
                Err(ApiError::WrongCredentials) => error.set(Some(msg_wrong)),
                // Verifying needs the current Secret Key from the keychain; on a
                // biometric-only vault it may be absent — point at the kit, not a raw error.
                Err(ApiError::Keychain(_)) => error.set(Some(msg_no_keychain)),
                Err(e) => error.set(Some(format!("{msg_err} {e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="flex items-center gap-1.5">
                    <div class="text-sm font-medium text-text-primary">
                        {move || t!(i18n, settings.change_password)}
                    </div>
                    <HelpPopover
                        body=Signal::derive(move || {
                            t_string!(i18n, settings.help_password_vs_rekey).to_owned()
                        })
                        label=Signal::derive(move || t_string!(i18n, settings.help_aria).to_owned())
                        testid="help-change-password"
                    />
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.change_password_desc)}
                </div>
                <Show when=move || stale.get() fallback=|| ()>
                    <div class="mt-1.5" data-testid="password-age-nudge">
                        <Badge variant=BadgeVariant::Warning size=BadgeSize::Sm>
                            {move || t!(i18n, settings.credential_nudge_password)}
                        </Badge>
                    </div>
                </Show>
            </div>
            <div class="shrink-0 mt-0.5">
                <Button
                    variant=Variant::Secondary
                    attr:data-testid="change-master-password"
                    on:click=move |_: web_sys::MouseEvent| open.run(())
                >
                    {move || t!(i18n, settings.change_password_button)}
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
                <DialogTitle>{move || t!(i18n, settings.change_password_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[22rem]">
                    <FormField
                        id="current-master-password"
                        label=Signal::derive(move || {
                            t_string!(i18n, settings.change_password_current).to_owned()
                        })
                    >
                        <Input
                            id="current-master-password"
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
                            id="new-master-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, settings.change_password_new).to_owned()
                            })
                        >
                            <Input
                                id="new-master-password"
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
                        id="new-master-password-confirm"
                        label=Signal::derive(move || {
                            t_string!(i18n, settings.change_password_confirm).to_owned()
                        })
                    >
                        <Input
                            id="new-master-password-confirm"
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

                </div>
            </DialogBody>
            <DialogFooter>
                <Button
                    variant=Variant::Ghost
                    size=Size::Sm
                    on:click=move |_: web_sys::MouseEvent| close.run(())
                >
                    {move || t!(i18n, settings.cancel)}
                </Button>
                {move || {
                    let loading = busy.get();
                    let disabled = !can_submit();
                    view! {
                        <Button
                            variant=Variant::Primary
                            size=Size::Sm
                            loading=loading
                            disabled=disabled
                            attr:data-testid="change-password-submit"
                            on:click=move |_: web_sys::MouseEvent| submit()
                        >
                            {move || t!(i18n, settings.change_password_submit)}
                        </Button>
                    }
                }}
            </DialogFooter>
        </Dialog>
    }
}
