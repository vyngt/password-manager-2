//! Settings ▸ Security ▸ "Recovery Key" row + enroll/revoke dialogs (slice 5.7).
//!
//! Enroll is a two-phase dialog: phase 1 re-auths the current password and mints the key;
//! phase 2 shows the `RK1-` **once** via `SecretDisplay` and offers to save the separate
//! Recovery Kit PDF (🔴 the copy insists it be kept apart from the Emergency Kit — together
//! they open the vault with no password). Revoke is an honest confirm: it protects only
//! future copies of the vault; truly revoking a leaked key is a re-key (5.8). Enrolled state
//! is loaded from `recovery_key_enrolled` (a derivative bool — the slot never crosses).

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::{Button, Input};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::secret_display::SecretDisplay;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn RecoveryKeySetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let enrolled = RwSignal::new(false);
    let dialog_open = RwSignal::new(false);
    let revoke_open = RwSignal::new(false);
    let current = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    // `None` → phase 1 (confirm); `Some(display)` → phase 2 (the minted key, shown once).
    let new_key = RwSignal::new(Option::<String>::None);
    // Whether the Recovery Kit PDF was saved this session. The `RK1-` key is show-once and —
    // unlike the Secret Key — is NOT re-derivable, so closing phase 2 without saving loses it
    // while recovery stays enrolled. `done_enroll` warns in that case.
    let kit_saved = RwSignal::new(false);

    let vault_path = move || active.path.get().unwrap_or_default();

    // Load enrolled state on mount (a derivative bool; the slot never leaves core).
    Effect::new(move |_| {
        let path = vault_path();
        if path.is_empty() {
            return;
        }
        spawn_local(async move {
            enrolled.set(
                api::recovery::recovery_key_enrolled(&path)
                    .await
                    .unwrap_or(false),
            );
        });
    });

    let open_enroll = Callback::new(move |()| {
        error.set(None);
        current.set(String::new());
        new_key.set(None);
        kit_saved.set(false);
        dialog_open.set(true);
    });
    // Close the enroll dialog. If we're in phase 2 (a key was minted) and the Recovery Kit was
    // never saved, warn first — the `RK1-` key can't be shown again.
    let close_enroll = Callback::new(move |()| {
        if new_key.get().is_some() && !kit_saved.get() {
            let dismiss = t_string!(i18n, settings.dismiss).to_owned();
            toast.show(
                ToastInput::new(t_string!(i18n, settings.recovery_key_unsaved).to_owned())
                    .variant(ToastVariant::Warning)
                    .dismiss_label(dismiss),
            );
        }
        new_key.set(None);
        kit_saved.set(false);
        dialog_open.set(false);
    });
    let open_revoke = Callback::new(move |()| {
        error.set(None);
        revoke_open.set(true);
    });
    let close_revoke = Callback::new(move |()| revoke_open.set(false));

    let confirm_enroll = move || {
        if busy.get() || current.get().is_empty() {
            return;
        }
        error.set(None);
        busy.set(true);
        let path = vault_path();
        let pw = current.get();
        let msg_wrong = t_string!(i18n, settings.recovery_key_wrong).to_owned();
        let msg_err = t_string!(i18n, settings.recovery_key_err).to_owned();
        let msg_no_keychain = t_string!(i18n, settings.kit_reexport_no_keychain).to_owned();
        spawn_local(async move {
            match api::recovery::enroll_recovery_key(&path, &pw).await {
                Ok(out) => {
                    current.set(String::new());
                    enrolled.set(true);
                    new_key.set(Some(out.recovery_key_display));
                }
                // The command re-verifies the current password first; a mismatch changes nothing.
                Err(ApiError::WrongCredentials) => error.set(Some(msg_wrong)),
                // Deriving KEK_rec needs the current Secret Key from the keychain; on a
                // biometric-only vault it may be absent — point at the kit, not a raw error.
                Err(ApiError::Keychain(_)) => error.set(Some(msg_no_keychain)),
                Err(e) => error.set(Some(format!("{msg_err} {e}"))),
            }
            busy.set(false);
        });
    };

    // Save the separate Recovery Kit PDF from the freshly-minted key (it is NOT stored, so the
    // display is passed through from `new_key`).
    let save_kit = move || {
        if busy.get() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        let display = untrack(|| new_key.get()).unwrap_or_default();
        if path.is_empty() || display.is_empty() {
            return;
        }
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        let dialog_title = untrack(|| t_string!(i18n, settings.recovery_key_save_kit).to_owned());
        let msg_saved = untrack(|| t_string!(i18n, settings.recovery_key_saved).to_owned());
        let msg_err = untrack(|| t_string!(i18n, settings.recovery_key_save_err).to_owned());
        busy.set(true);
        spawn_local(async move {
            let opts = SaveDialogOptions {
                title: Some(dialog_title),
                default_path: Some("vedge-recovery-kit.pdf".to_owned()),
                filters: vec![DialogFilter {
                    name: "PDF".to_owned(),
                    extensions: vec!["pdf".to_owned()],
                }],
            };
            match api::dialog::save(&opts).await {
                Ok(Some(dest)) => {
                    let (msg, variant) =
                        match api::recovery::write_recovery_kit_pdf(&path, &display, &dest).await {
                            Ok(()) => {
                                kit_saved.set(true);
                                (msg_saved, ToastVariant::Success)
                            }
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

    let confirm_revoke = move || {
        if busy.get() {
            return;
        }
        error.set(None);
        busy.set(true);
        let path = vault_path();
        let msg_err = t_string!(i18n, settings.recovery_key_revoke_err).to_owned();
        spawn_local(async move {
            match api::recovery::revoke_recovery_key(&path).await {
                Ok(()) => {
                    enrolled.set(false);
                    revoke_open.set(false);
                }
                Err(e) => error.set(Some(format!("{msg_err} {e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.recovery_key)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || {
                        if enrolled.get() {
                            t_string!(i18n, settings.recovery_key_on).to_owned()
                        } else {
                            t_string!(i18n, settings.recovery_key_desc).to_owned()
                        }
                    }}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                <Show
                    when=move || enrolled.get()
                    fallback=move || {
                        view! {
                            <Button
                                variant=Variant::Secondary
                                attr:data-testid="recovery-setup"
                                on:click=move |_: web_sys::MouseEvent| open_enroll.run(())
                            >
                                {move || t!(i18n, settings.recovery_key_setup_button)}
                            </Button>
                        }
                    }
                >
                    <Button
                        variant=Variant::Ghost
                        attr:data-testid="recovery-turn-off"
                        on:click=move |_: web_sys::MouseEvent| open_revoke.run(())
                    >
                        {move || t!(i18n, settings.recovery_key_turn_off_button)}
                    </Button>
                </Show>
            </div>
        </div>

        <Dialog
            open=Signal::derive(move || dialog_open.get())
            on_close=close_enroll
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.recovery_key_setup_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[22rem]">
                    <div class="flex flex-col gap-2 rounded-lg border border-border bg-foreground/5 p-3 text-xs text-text-secondary">
                        {move || t!(i18n, settings.recovery_key_keep_separate)}
                    </div>

                    <Show
                        when=move || new_key.get().is_none()
                        fallback=move || {
                            view! {
                                <p class="text-sm text-text-primary">
                                    {move || t!(i18n, settings.recovery_key_new_intro)}
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
                                                attr:data-testid="recovery-save-kit"
                                                on:click=move |_: web_sys::MouseEvent| save_kit()
                                            >
                                                {move || t!(i18n, settings.recovery_key_save_kit)}
                                            </Button>
                                        }
                                    }}
                                    <Button
                                        variant=Variant::Primary
                                        size=Size::Sm
                                        attr:data-testid="recovery-done"
                                        on:click=move |_: web_sys::MouseEvent| close_enroll.run(())
                                    >
                                        {move || t!(i18n, settings.recovery_key_done)}
                                    </Button>
                                </div>
                            }
                        }
                    >
                        <FormField
                            id="recovery-current-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, settings.recovery_key_current).to_owned()
                            })
                        >
                            <Input
                                id="recovery-current-password"
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
                                on:click=move |_: web_sys::MouseEvent| close_enroll.run(())
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
                                        attr:data-testid="recovery-setup-confirm"
                                        on:click=move |_: web_sys::MouseEvent| confirm_enroll()
                                    >
                                        {move || t!(i18n, settings.recovery_key_setup_confirm)}
                                    </Button>
                                }
                            }}
                        </div>
                    </Show>
                </div>
            </DialogBody>
        </Dialog>

        <Dialog
            open=Signal::derive(move || revoke_open.get())
            on_close=close_revoke
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.recovery_key_revoke_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 min-w-[22rem]">
                    <p class="text-sm text-text-secondary">
                        {move || t!(i18n, settings.recovery_key_revoke_body)}
                    </p>
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
                            on:click=move |_: web_sys::MouseEvent| close_revoke.run(())
                        >
                            {move || t!(i18n, settings.cancel)}
                        </Button>
                        {move || {
                            let loading = busy.get();
                            view! {
                                <Button
                                    variant=Variant::Danger
                                    size=Size::Sm
                                    loading=loading
                                    attr:data-testid="recovery-revoke-confirm"
                                    on:click=move |_: web_sys::MouseEvent| confirm_revoke()
                                >
                                    {move || t!(i18n, settings.recovery_key_revoke_confirm)}
                                </Button>
                            }
                        }}
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
