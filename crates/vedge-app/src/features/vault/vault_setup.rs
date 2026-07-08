//! Create-vault onboarding wizard.
//!
//! Location → master password → create → Emergency Kit (acknowledged) → land
//! unlocked at `/v/vault`. Follows the app's `spawn_local` + `use_navigate`
//! IPC idiom (see `pages/page.rs`). Copy is localized via the `onboarding`
//! i18n namespace. Locale-dependent strings used inside `spawn_local` are read
//! in the handler body first (a `spawn_local` future has no reactive owner).

use leptos::either::EitherOf3;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uuid::Uuid;

use vedge_ipc::{CreateVaultInputDto, RecentVaultDto};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::password_strength::score as password_score;
use crate::features::vault::secret_display::SecretDisplay;
use crate::features::vault::vault_launch::display_name_from_path;
use crate::i18n::*;

use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, Checkbox, Input, PasswordStrengthMeter, Step, StepIndicator};
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

/// Minimum strength (0–4) required to leave the password step.
const MIN_STRENGTH: u8 = 2;

#[component]
pub fn VaultSetup() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, onboarding.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, onboarding.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };

    let current_step = RwSignal::new(0usize);
    let path = RwSignal::new(String::new());
    let pw = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    let created = RwSignal::new(false);
    let secret = RwSignal::new(String::new());
    let keychain_ok = RwSignal::new(true);
    let acknowledged = RwSignal::new(false);

    let strength = Memo::new(move |_| password_score(&pw.get()));

    let can_next_location = move || !path.get().trim().is_empty();
    let password_valid = move || {
        let p = pw.get();
        !p.is_empty() && p == confirm.get() && strength.get() >= MIN_STRENGTH
    };
    let can_finish = move || created.get() && acknowledged.get();

    // `Signal::derive` (not `stored`) so the `t_string!` reads run inside a
    // reactive context — an eager read in the component body is "outside a
    // reactive tracking context" — and so labels relocalize on language switch.
    let steps = Signal::derive(move || {
        vec![
            Step::new(t_string!(i18n, onboarding.step_location).to_string()),
            Step::new(t_string!(i18n, onboarding.step_password).to_string()),
            Step::new(t_string!(i18n, onboarding.step_kit).to_string()),
        ]
    });

    view! {
        <div class="mx-auto flex h-full w-full max-w-xl flex-col gap-6 p-8">
            <h1 class="text-xl font-semibold text-text-primary">
                {move || t!(i18n, onboarding.heading)}
            </h1>
            <StepIndicator steps=steps current_step=current_step />

            {move || match current_step.get() {
                // ---- Step 1: location -------------------------------------
                0 => EitherOf3::A(view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-1">
                            <span class="text-sm text-text-secondary">
                                {move || t!(i18n, onboarding.location_label)}
                            </span>
                            <div class="flex gap-2">
                                <Input
                                    id="vault-path"
                                    placeholder=Signal::derive(move || {
                                        t_string!(i18n, onboarding.location_placeholder).to_string()
                                    })
                                    value=Signal::derive(move || path.get())
                                    on_input=Callback::new(move |v: String| path.set(v))
                                    class="flex-1"
                                />
                                <Button
                                    variant=Variant::Secondary
                                    on:click=move |_| {
                                        let dialog_title =
                                            t_string!(i18n, onboarding.choose_dialog_title)
                                                .to_string();
                                        spawn_local(async move {
                                            let opts = SaveDialogOptions {
                                                title: Some(dialog_title),
                                                default_path: Some("my-vault.vdb".to_string()),
                                                filters: vec![DialogFilter {
                                                    name: "VEdge Vault".to_string(),
                                                    extensions: vec!["vdb".to_string()],
                                                }],
                                            };
                                            match api::dialog::save(&opts).await {
                                                Ok(Some(p)) => path.set(p),
                                                Ok(None) => {}
                                                Err(e) => web_sys::console::error_1(
                                                    &format!("dialog failed: {e:?}").into(),
                                                ),
                                            }
                                        });
                                    }
                                >
                                    {move || t!(i18n, onboarding.choose)}
                                </Button>
                            </div>
                        </div>
                        <div class="flex justify-end">
                            {move || {
                                let enabled = can_next_location();
                                view! {
                                    <Button
                                        variant=Variant::Primary
                                        disabled=!enabled
                                        on:click=move |_| current_step.set(1)
                                    >
                                        {move || t!(i18n, onboarding.next)}
                                    </Button>
                                }
                            }}
                        </div>
                    </div>
                }),
                // ---- Step 2: master password ------------------------------
                1 => EitherOf3::B(view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-2">
                            <span class="text-sm text-text-secondary">
                                {move || t!(i18n, onboarding.password_label)}
                            </span>
                            <Input
                                id="master-password"
                                input_type="password"
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, onboarding.password_placeholder).to_string()
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
                            <PasswordStrengthMeter
                                score=strength
                                strength_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.strength_label).to_string()
                                })
                                level_labels=Signal::derive(move || [
                                    t_string!(i18n, onboarding.strength_weak).to_string(),
                                    t_string!(i18n, onboarding.strength_fair).to_string(),
                                    t_string!(i18n, onboarding.strength_good).to_string(),
                                    t_string!(i18n, onboarding.strength_strong).to_string(),
                                ])
                            />
                            <Input
                                id="master-password-confirm"
                                input_type="password"
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, onboarding.confirm_placeholder).to_string()
                                })
                                value=Signal::derive(move || confirm.get())
                                on_input=Callback::new(move |v: String| confirm.set(v))
                                reveal_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.show_password).to_string()
                                })
                                hide_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.hide_password).to_string()
                                })
                            />
                        </div>
                        <div class="flex justify-between">
                            <Button variant=Variant::Ghost on:click=move |_| current_step.set(0)>
                                {move || t!(i18n, onboarding.back)}
                            </Button>
                            {move || {
                                let enabled = password_valid() && !creating.get();
                                view! {
                                    <Button
                                        variant=Variant::Primary
                                        disabled=!enabled
                                        on:click=move |_| {
                                            if creating.get() {
                                                return;
                                            }
                                            creating.set(true);
                                            let vault_path = path.get();
                                            let master_password = pw.get();
                                            let msg_exists =
                                                t_string!(i18n, onboarding.err_exists).to_string();
                                            let msg_create =
                                                t_string!(i18n, onboarding.err_create).to_string();
                                            spawn_local(async move {
                                                let input = CreateVaultInputDto {
                                                    vault_path,
                                                    master_password,
                                                    secret_key_b64: None,
                                                };
                                                match api::vault::create(&input).await {
                                                    Ok(out) => {
                                                        secret.set(out.secret_key_display);
                                                        keychain_ok.set(out.keychain_stored);
                                                        created.set(true);
                                                        current_step.set(2);
                                                        // Add to recents so it appears on the
                                                        // launch screen next time (non-fatal).
                                                        let dto = RecentVaultDto {
                                                            id: Uuid::new_v4().to_string(),
                                                            path: input.vault_path.clone(),
                                                            display_name: display_name_from_path(
                                                                &input.vault_path,
                                                            ),
                                                            last_opened: None,
                                                            sort_order: 0,
                                                        };
                                                        let _ = api::recent::add_recent_vault(&dto)
                                                            .await;
                                                    }
                                                    Err(ApiError::AlreadyExists) => {
                                                        show_error(msg_exists);
                                                    }
                                                    Err(e) => {
                                                        show_error(format!("{msg_create}{e}"));
                                                    }
                                                }
                                                creating.set(false);
                                            });
                                        }
                                    >
                                        {move || t!(i18n, onboarding.create)}
                                    </Button>
                                }
                            }}
                        </div>
                    </div>
                }),
                // ---- Step 3: Emergency Kit --------------------------------
                _ => EitherOf3::C(view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-2">
                            <span class="text-sm text-text-secondary">
                                {move || t!(i18n, onboarding.secret_intro)}
                            </span>
                            <SecretDisplay value=Signal::derive(move || secret.get()) />
                        </div>
                        {move || (!keychain_ok.get()).then(|| view! {
                            <p class="text-sm text-warning-text">
                                {move || t!(i18n, onboarding.keychain_warning)}
                            </p>
                        })}
                        <div class="flex flex-wrap items-center gap-2">
                            <Button
                                variant=Variant::Secondary
                                on:click=move |_| {
                                    let vault_path = path.get();
                                    let dialog_title =
                                        t_string!(i18n, onboarding.save_kit_dialog_title)
                                            .to_string();
                                    let msg_saved =
                                        t_string!(i18n, onboarding.kit_saved).to_string();
                                    let msg_kit_err =
                                        t_string!(i18n, onboarding.err_kit_save).to_string();
                                    let msg_dialog_err =
                                        t_string!(i18n, onboarding.err_dialog).to_string();
                                    spawn_local(async move {
                                        let opts = SaveDialogOptions {
                                            title: Some(dialog_title),
                                            default_path: Some(
                                                "vedge-emergency-kit.pdf".to_string(),
                                            ),
                                            filters: vec![DialogFilter {
                                                name: "PDF".to_string(),
                                                extensions: vec!["pdf".to_string()],
                                            }],
                                        };
                                        match api::dialog::save(&opts).await {
                                            Ok(Some(dest)) => {
                                                match api::emergency_kit::write_pdf(
                                                    &vault_path,
                                                    &dest,
                                                )
                                                .await
                                                {
                                                    Ok(()) => show_success(msg_saved),
                                                    Err(e) => {
                                                        show_error(format!("{msg_kit_err}{e}"))
                                                    }
                                                }
                                            }
                                            Ok(None) => {}
                                            Err(e) => {
                                                show_error(format!("{msg_dialog_err}{e}"))
                                            }
                                        }
                                    });
                                }
                            >
                                {move || t!(i18n, onboarding.download_kit)}
                            </Button>
                        </div>
                        <div class="flex items-center gap-2 text-sm text-text-primary">
                            <Checkbox
                                checked=Signal::derive(move || acknowledged.get())
                                on_change=Callback::new(move |v: bool| acknowledged.set(v))
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, onboarding.ack_aria).to_string()
                                })
                            />
                            <span>{move || t!(i18n, onboarding.ack_label)}</span>
                        </div>
                        <div class="flex justify-end">
                            {move || {
                                let enabled = can_finish();
                                view! {
                                    <Button
                                        variant=Variant::Primary
                                        disabled=!enabled
                                        on:click=move |_| {
                                            active.path.set(Some(path.get()));
                                            let nav = use_navigate();
                                            nav("/v/vault", Default::default());
                                        }
                                    >
                                        {move || t!(i18n, onboarding.finish)}
                                    </Button>
                                }
                            }}
                        </div>
                    </div>
                }),
            }}
        </div>
    }
}
