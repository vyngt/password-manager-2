//! Emergency Kit re-export (slice 5.1) — a Settings → Security row.
//!
//! Lets an unlocked vault re-download its printable PDF kit (containing the
//! Secret Key). Pure wiring over the already-registered `write_emergency_kit_pdf`
//! command, reusing the same native save-dialog flow as onboarding (Step 3).
//! The `write_pdf` command reads the Secret Key from the OS keychain, so on a
//! device whose keychain entry is gone it surfaces `ApiError::Keychain` with a
//! pointed message rather than a raw error.

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::api::error::ApiError;
use crate::features::help_popover::HelpPopover;
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn EmergencyKitSetting() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let busy = RwSignal::new(false);

    let reexport = move || {
        if busy.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        // Hoist the localized strings out of the async block (owner-safe).
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
                        // The command reads the Secret Key from the OS keychain; a
                        // missing entry points the user at recovery, not a raw error.
                        Err(ApiError::Keychain(_)) => (msg_no_keychain, ToastVariant::Danger),
                        Err(e) => (format!("{msg_err}{e}"), ToastVariant::Danger),
                    };
                    toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
                }
                // User cancelled the save dialog — silent.
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
                        {move || t!(i18n, settings.kit_reexport)}
                    </div>
                    <HelpPopover
                        body=Signal::derive(move || t_string!(i18n, unlock.help_ek_a).to_owned())
                        label=Signal::derive(move || t_string!(i18n, settings.help_aria).to_owned())
                        testid="help-emergency-kit"
                    />
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.kit_reexport_desc)}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                {move || {
                    let is_busy = busy.get();
                    view! {
                        <Button
                            variant=Variant::Secondary
                            loading=is_busy
                            attr:data-testid="kit-reexport"
                            on:click=move |_: web_sys::MouseEvent| reexport()
                        >
                            {move || t!(i18n, settings.kit_reexport_button)}
                        </Button>
                    }
                }}
            </div>
        </div>
    }
}
