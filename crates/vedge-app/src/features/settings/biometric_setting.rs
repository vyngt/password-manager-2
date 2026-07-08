//! Settings → Security → "Biometric unlock" row + enroll dialog.
//!
//! Biometric enrollment is **per-vault** and lives in the OS/core (not in the app-global
//! `SecurityPrefs`), so this component keeps its own local `available`/`enrolled` state
//! loaded from `api::biometric`. Turning the toggle on opens the enroll dialog (mockup
//! 2.8b: three honest security notes + a master-password re-prompt); turning it off
//! disables. The re-prompt is verified server-side against the vault's `verify_hash`.

use crate::api;
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use icondata as i;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::toggle::Toggle;
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

#[component]
pub fn BiometricSetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();

    let available = RwSignal::new(false);
    let enrolled = RwSignal::new(false);
    let dialog_open = RwSignal::new(false);
    let pw = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Settings always runs inside an unlocked vault; the path drives every command.
    let vault_path = move || active.path.get().unwrap_or_default();

    // Load device availability + this vault's enrollment on mount.
    Effect::new(move |_| {
        let path = vault_path();
        spawn_local(async move {
            let avail = api::biometric::available().await.unwrap_or(false);
            available.set(avail);
            if avail && !path.is_empty() {
                enrolled.set(api::biometric::is_enrolled(&path).await.unwrap_or(false));
            }
        });
    });

    let close = Callback::new(move |()| dialog_open.set(false));

    // Toggle: turning on opens the enroll dialog (KEK is only stored after the
    // master-password re-prompt succeeds); turning off disables immediately.
    let on_toggle = Callback::new(move |v: bool| {
        if v {
            error.set(None);
            pw.set(String::new());
            dialog_open.set(true);
        } else {
            let path = vault_path();
            spawn_local(async move {
                let _ = api::biometric::disable(&path).await;
                enrolled.set(false);
            });
        }
    });

    // Confirm enroll: verify the re-prompted master password + store the KEK.
    let do_enroll = move || {
        if busy.get() {
            return;
        }
        let password = pw.get();
        if password.is_empty() {
            return;
        }
        error.set(None);
        busy.set(true);
        let path = vault_path();
        let msg_wrong = t_string!(i18n, settings.biometric_wrong_password).to_string();
        let msg_err = t_string!(i18n, settings.biometric_error).to_string();
        spawn_local(async move {
            match api::biometric::enroll(&path, &password).await {
                Ok(()) => {
                    enrolled.set(true);
                    dialog_open.set(false);
                    pw.set(String::new());
                }
                Err(ApiError::WrongCredentials) => error.set(Some(msg_wrong)),
                Err(e) => error.set(Some(format!("{msg_err} {e}"))),
            }
            busy.set(false);
        });
    };

    view! {
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.biometric)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || {
                        if available.get() {
                            t_string!(i18n, settings.biometric_desc).to_string()
                        } else {
                            t_string!(i18n, settings.biometric_unavailable).to_string()
                        }
                    }}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                {move || {
                    let dis = !available.get();
                    view! {
                        <Toggle
                            checked=Signal::derive(move || enrolled.get())
                            on_change=on_toggle
                            disabled=dis
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, settings.biometric).to_string()
                            })
                        />
                    }
                }}
            </div>
        </div>

        <Dialog
            open=Signal::derive(move || dialog_open.get())
            on_close=close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.cancel).to_string())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.biometric_enroll_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-4 min-w-[22rem]">
                    <p class="text-sm text-foreground/70">
                        {move || t!(i18n, settings.biometric_enroll_body)}
                    </p>
                    <div class="flex flex-col gap-2 rounded-lg border border-border bg-foreground/5 p-3 text-xs text-text-secondary">
                        <div class="flex gap-2">
                            <span class="text-primary shrink-0 mt-0.5">
                                <Icon attr:aria-hidden="true" icon=i::FaMicrochipSolid />
                            </span>
                            <span>{move || t!(i18n, settings.biometric_note_hw)}</span>
                        </div>
                        <div class="flex gap-2">
                            <span class="text-primary shrink-0 mt-0.5">
                                <Icon attr:aria-hidden="true" icon=i::FaKeySolid />
                            </span>
                            <span>{move || t!(i18n, settings.biometric_note_fallback)}</span>
                        </div>
                        <div class="flex gap-2">
                            <span class="text-primary shrink-0 mt-0.5">
                                <Icon attr:aria-hidden="true" icon=i::FaDesktopSolid />
                            </span>
                            <span>{move || t!(i18n, settings.biometric_note_device)}</span>
                        </div>
                    </div>
                    <Input
                        id="biometric-master-password"
                        input_type="password"
                        placeholder=Signal::derive(move || {
                            t_string!(i18n, unlock.master_password).to_string()
                        })
                        value=Signal::derive(move || pw.get())
                        on_input=Callback::new(move |v: String| pw.set(v))
                        reveal_label=Signal::derive(move || {
                            t_string!(i18n, onboarding.show_password).to_string()
                        })
                        hide_label=Signal::derive(move || {
                            t_string!(i18n, onboarding.hide_password).to_string()
                        })
                    />
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
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    size=Size::Sm
                                    loading=loading
                                    on:click=move |_: web_sys::MouseEvent| do_enroll()
                                >
                                    {move || t!(i18n, settings.biometric_enable)}
                                </Button>
                            }
                        }}
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
