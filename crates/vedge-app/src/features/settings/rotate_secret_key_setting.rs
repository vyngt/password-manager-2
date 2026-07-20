//! Rotate-Secret-Key (slice 5.6 ③) — a Settings ▸ Security row + two-phase dialog.
//!
//! Rotating the Secret Key changes what unlocks the vault going forward. 🔴 The
//! old Emergency Kit is **not** orphaned — it stays the only key to every backup
//! and snapshot taken *before* the rotation, so the dialog tells the user to keep
//! it (never "shred it"). Phase 1 re-authenticates the current password and
//! rotates (the fresh key is generated server-side); phase 2 shows the new key
//! once and offers to re-issue the Emergency Kit from it. `busy` gates the
//! O(n) rewrap against a double-submit.

use chrono::Utc;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::{Badge, Button, Input};
use vedge_ui::primitives::tokens::{
    BadgeSize, BadgeVariant, DialogSize, Size, ToastVariant, Variant,
};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::api::error::ApiError;
use crate::features::help_popover::HelpPopover;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::secret_display::SecretDisplay;
use crate::features::vault::timestamps::is_older_than_days;
use crate::i18n::{t, t_string, use_i18n};

/// Nudge to rotate the Secret Key after this many days (slice 5.9 ③). Longer than the password
/// threshold: the SK is 256-bit random, so age is a compromise-window concern, not a strength one.
const SECRET_KEY_STALE_DAYS: i64 = 730;

#[component]
pub fn RotateSecretKeySetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let dialog_open = RwSignal::new(false);
    let current = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    // `None` → phase 1 (confirm); `Some(display)` → phase 2 (the re-issued key).
    let new_key = RwSignal::new(Option::<String>::None);
    // Credential-age nudge (slice 5.9 ③): true when the Secret Key is stale.
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
                    &status.secret_key_rotated_at,
                    now_ms,
                    SECRET_KEY_STALE_DAYS,
                ));
            }
        });
    });

    let vault_path = move || active.path.get().unwrap_or_default();

    let open = Callback::new(move |()| {
        error.set(None);
        current.set(String::new());
        new_key.set(None);
        dialog_open.set(true);
    });
    let close = Callback::new(move |()| dialog_open.set(false));

    let confirm_rotate = move || {
        if busy.get() || current.get().is_empty() {
            return;
        }
        error.set(None);
        busy.set(true);
        let path = vault_path();
        let pw = current.get();
        let msg_wrong = t_string!(i18n, settings.rotate_wrong).to_owned();
        let msg_err = t_string!(i18n, settings.rotate_err).to_owned();
        let msg_no_keychain = t_string!(i18n, settings.kit_reexport_no_keychain).to_owned();
        spawn_local(async move {
            match api::password::rotate_secret_key(&path, &pw).await {
                Ok(out) => {
                    current.set(String::new());
                    new_key.set(Some(out.secret_key_display));
                    // Just rotated → no longer stale.
                    stale.set(false);
                }
                // The command re-verifies the current password first; a mismatch
                // is `WrongCredentials` and nothing was rotated.
                Err(ApiError::WrongCredentials) => error.set(Some(msg_wrong)),
                // Verifying needs the current Secret Key from the keychain; on a
                // biometric-only vault it may be absent — point at the kit, not a raw error.
                Err(ApiError::Keychain(_)) => error.set(Some(msg_no_keychain)),
                Err(e) => error.set(Some(format!("{msg_err} {e}"))),
            }
            busy.set(false);
        });
    };

    // Re-issue the Emergency Kit for the freshly rotated key — same save-dialog
    // flow as EmergencyKitSetting. `write_pdf` reads the *new* key back from the
    // keychain (change_password stored it during the rotation).
    let save_kit = move || {
        if busy.get() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        let dialog_title = untrack(|| t_string!(i18n, settings.kit_reexport).to_owned());
        let msg_saved = untrack(|| t_string!(i18n, settings.kit_reexport_saved).to_owned());
        let msg_no_keychain =
            untrack(|| t_string!(i18n, settings.kit_reexport_no_keychain).to_owned());
        let msg_err = untrack(|| t_string!(i18n, settings.kit_reexport_err).to_owned());
        busy.set(true);
        spawn_local(async move {
            let opts = SaveDialogOptions {
                title: Some(dialog_title),
                default_path: Some("vedge-emergency-kit.pdf".to_owned()),
                filters: vec![DialogFilter {
                    name: "PDF".to_owned(),
                    extensions: vec!["pdf".to_owned()],
                }],
            };
            match api::dialog::save(&opts).await {
                Ok(Some(dest)) => {
                    let (msg, variant) = match api::emergency_kit::write_pdf(&path, &dest).await {
                        Ok(()) => (msg_saved, ToastVariant::Success),
                        Err(ApiError::Keychain(_)) => (msg_no_keychain, ToastVariant::Danger),
                        Err(e) => (format!("{msg_err}{e}"), ToastVariant::Danger),
                    };
                    toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
                }
                Ok(None) => {}
                Err(e) => {
                    toast.show(
                        ToastInput::new(format!("{msg_err}{e}"))
                            .variant(ToastVariant::Danger)
                            .dismiss_label(dismiss),
                    );
                }
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="flex items-center gap-1.5">
                    <div class="text-sm font-medium text-text-primary">
                        {move || t!(i18n, settings.rotate_secret_key)}
                    </div>
                    <HelpPopover
                        body=Signal::derive(move || {
                            t_string!(i18n, settings.rotate_keep_old_kit).to_owned()
                        })
                        label=Signal::derive(move || t_string!(i18n, settings.help_aria).to_owned())
                        testid="help-rotate-secret-key"
                    />
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.rotate_secret_key_desc)}
                </div>
                <Show when=move || stale.get() fallback=|| ()>
                    <div class="mt-1.5" data-testid="secret-key-age-nudge">
                        <Badge variant=BadgeVariant::Warning size=BadgeSize::Sm>
                            {move || t!(i18n, settings.credential_nudge_secret_key)}
                        </Badge>
                    </div>
                </Show>
            </div>
            <div class="shrink-0 mt-0.5">
                <Button
                    variant=Variant::Secondary
                    attr:data-testid="rotate-secret-key"
                    on:click=move |_: web_sys::MouseEvent| open.run(())
                >
                    {move || t!(i18n, settings.rotate_secret_key_button)}
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
                <DialogTitle>{move || t!(i18n, settings.rotate_secret_key_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[22rem]">
                    // 🔴 The reversed "keep your old kit" copy — shown in both phases.
                    <div class="flex flex-col gap-2 rounded-lg border border-border bg-foreground/5 p-3 text-xs text-text-secondary">
                        {move || t!(i18n, settings.rotate_keep_old_kit)}
                    </div>

                    <Show
                        when=move || new_key.get().is_none()
                        fallback=move || {
                            view! {
                                <p class="text-sm text-text-primary">
                                    {move || t!(i18n, settings.rotate_new_key_intro)}
                                </p>
                                <SecretDisplay value=Signal::derive(move || {
                                    new_key.get().unwrap_or_default()
                                }) />
                                <div class="flex justify-end gap-2">
                                    {move || {
                                        let loading = busy.get();
                                        view! {
                                            <Button
                                                variant=Variant::Secondary
                                                size=Size::Sm
                                                loading=loading
                                                attr:data-testid="rotate-save-kit"
                                                on:click=move |_: web_sys::MouseEvent| save_kit()
                                            >
                                                {move || t!(i18n, settings.rotate_save_kit)}
                                            </Button>
                                        }
                                    }}
                                    <Button
                                        variant=Variant::Primary
                                        size=Size::Sm
                                        attr:data-testid="rotate-done"
                                        on:click=move |_: web_sys::MouseEvent| close.run(())
                                    >
                                        {move || t!(i18n, settings.rotate_done)}
                                    </Button>
                                </div>
                            }
                        }
                    >
                        <FormField
                            id="rotate-master-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, settings.rotate_current).to_owned()
                            })
                        >
                            <Input
                                id="rotate-master-password"
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
                                let loading = busy.get();
                                let disabled = current.get().is_empty();
                                view! {
                                    <Button
                                        variant=Variant::Primary
                                        size=Size::Sm
                                        loading=loading
                                        disabled=disabled
                                        attr:data-testid="rotate-secret-key-confirm"
                                        on:click=move |_: web_sys::MouseEvent| confirm_rotate()
                                    >
                                        {move || t!(i18n, settings.rotate_confirm)}
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
