//! The forced "set a new master password" dialog after a recovery unlock (slice 5.7 ⑤).
//!
//! Shown on the launch screen the moment `unlock_with_recovery_key` succeeds: the session is
//! unlocked but pending — the user forgot the old password, so recovery ends by choosing a new
//! one (which also nulls the now-defunct recovery slot). The view lives here; the async submit
//! (`change_password_after_recovery` → navigate into the vault) stays in the orchestrator.

use leptos::prelude::*;
use vedge_ui::components::feedback::dialog::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::form::form_field::FormField;
use vedge_ui::components::{Button, Input, PasswordStrengthMeter};
use vedge_ui::primitives::tokens::{DialogSize, Size, Variant};

use crate::features::vault::password_strength::score as password_score;
use crate::i18n::{t, t_string, use_i18n};

/// Minimum acceptable strength (0–4) — mirrors onboarding + change-password.
const MIN_STRENGTH: u8 = 2;

#[component]
pub fn RecoveryResetDialog(
    open: RwSignal<bool>,
    new_password: RwSignal<String>,
    confirm_password: RwSignal<String>,
    /// Reuses the launch screen's `unlocking` busy signal.
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_submit: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    let strength = Memo::new(move |_| password_score(&new_password.get()));
    let can_submit = move || {
        !new_password.get().is_empty()
            && new_password.get() == confirm_password.get()
            && strength.get() >= MIN_STRENGTH
    };

    view! {
        <Dialog
            open=Signal::derive(move || open.get())
            on_close=on_close
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, unlock.dismiss).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, unlock.recover_reset_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-4 pb-4 min-w-[22rem]">
                    <p class="text-sm text-text-secondary">
                        {move || t!(i18n, unlock.recover_reset_body)}
                    </p>

                    <div class="flex flex-col gap-1.5">
                        <FormField
                            id="recovery-new-password"
                            label=Signal::derive(move || {
                                t_string!(i18n, unlock.recover_reset_new).to_owned()
                            })
                        >
                            <Input
                                id="recovery-new-password"
                                input_type="password"
                                value=Signal::derive(move || new_password.get())
                                on_input=Callback::new(move |v: String| new_password.set(v))
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
                        id="recovery-confirm-password"
                        label=Signal::derive(move || {
                            t_string!(i18n, unlock.recover_reset_confirm_label).to_owned()
                        })
                    >
                        <Input
                            id="recovery-confirm-password"
                            input_type="password"
                            value=Signal::derive(move || confirm_password.get())
                            on_input=Callback::new(move |v: String| confirm_password.set(v))
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

                    <div class="flex justify-end">
                        {move || {
                            let loading = busy.get();
                            let disabled = !can_submit();
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    size=Size::Md
                                    loading=loading
                                    disabled=disabled
                                    attr:data-testid="recovery-reset-submit"
                                    on:click=move |_: web_sys::MouseEvent| on_submit.run(())
                                >
                                    {move || t!(i18n, unlock.recover_reset_submit)}
                                </Button>
                            }
                        }}
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}
