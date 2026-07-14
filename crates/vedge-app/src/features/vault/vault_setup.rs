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
use crate::api::dialog::{DialogFilter, OpenDialogOptions, SaveDialogOptions};
use crate::api::error::ApiError;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::password_strength::score as password_score;
use crate::features::vault::secret_display::SecretDisplay;
use crate::features::vault::vault_launch::display_name_from_path;
use crate::i18n::{t, t_string, use_i18n};

use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, Checkbox, Input, PasswordStrengthMeter, Step, StepIndicator};
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

/// Ensure the create location is a `.vedge` home directory (slice 5.2.0). A user may type
/// a bare name or a legacy `.vdb` in the location field; both become a `.vedge` home. An
/// already-`.vedge` path passes through unchanged.
fn ensure_vedge_home(input: &str) -> String {
    let trimmed = input.trim();
    match std::path::Path::new(trimmed)
        .extension()
        .and_then(|s| s.to_str())
    {
        Some("vedge") => trimmed.to_owned(),
        // A legacy `.vdb` typed in the field → swap the extension for `.vedge`.
        Some("vdb") => format!("{}.vedge", trimmed.strip_suffix(".vdb").unwrap_or(trimmed)),
        _ => format!("{trimmed}.vedge"),
    }
}

/// Minimum strength (0–4) required to leave the password step.
const MIN_STRENGTH: u8 = 2;

#[component]
pub fn VaultSetup() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, onboarding.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, onboarding.dismiss).to_owned());
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
            Step::new(t_string!(i18n, onboarding.step_location).to_owned()),
            Step::new(t_string!(i18n, onboarding.step_password).to_owned()),
            Step::new(t_string!(i18n, onboarding.step_kit).to_owned()),
        ]
    });

    view! {
        <div class="mx-auto flex h-full w-full max-w-xl flex-col gap-6 overflow-y-auto p-8">
            <h1 class="text-xl font-semibold text-text-primary">
                {move || t!(i18n, onboarding.heading)}
            </h1>
            <StepIndicator steps=steps current_step=current_step />

            {move || match current_step.get() {
                0 => {
                    EitherOf3::A(
                        // ---- Step 1: location -------------------------------------
                        view! {
                            <div class="flex flex-col gap-4">
                                <div class="flex flex-col gap-1">
                                    <span class="text-sm text-text-secondary">
                                        {move || t!(i18n, onboarding.location_label)}
                                    </span>
                                    <div class="flex gap-2">
                                        <Input
                                            id="vault-path"
                                            placeholder=Signal::derive(move || {
                                                t_string!(i18n, onboarding.location_placeholder).to_owned()
                                            })
                                            value=Signal::derive(move || path.get())
                                            on_input=Callback::new(move |v: String| path.set(v))
                                            class="flex-1"
                                        />
                                        <Button
                                            variant=Variant::Secondary
                                            on:click=move |_| {
                                                let dialog_title = t_string!(
                                                    i18n, onboarding.choose_dialog_title
                                                )
                                                    .to_owned();
                                                spawn_local(async move {
                                                    let opts = OpenDialogOptions {
                                                        title: Some(dialog_title),
                                                        filters: vec![],
                                                        directory: true,
                                                    };
                                                    match api::dialog::open(&opts).await {
                                                        Ok(Some(parent)) => {
                                                            let sep = parent.trim_end_matches(['/', '\\']);
                                                            path.set(format!("{sep}/my-vault.vedge"));
                                                        }
                                                        Ok(None) => {}
                                                        Err(e) => {
                                                            web_sys::console::error_1(
                                                                &format!("dialog failed: {e:?}").into(),
                                                            );
                                                        }
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
                                                attr:data-testid="onboarding-next"
                                                on:click=move |_| current_step.set(1)
                                            >
                                                {move || t!(i18n, onboarding.next)}
                                            </Button>
                                        }
                                    }}
                                </div>
                            </div>
                        },
                    )
                }
                1 => {
                    EitherOf3::B(
                        // ---- Step 2: master password ------------------------------
                        view! {
                            <div class="flex flex-col gap-4">
                                <div class="flex flex-col gap-2">
                                    <span class="text-sm text-text-secondary">
                                        {move || t!(i18n, onboarding.password_label)}
                                    </span>
                                    <Input
                                        id="master-password"
                                        input_type="password"
                                        placeholder=Signal::derive(move || {
                                            t_string!(i18n, onboarding.password_placeholder).to_owned()
                                        })
                                        value=Signal::derive(move || pw.get())
                                        on_input=Callback::new(move |v: String| pw.set(v))
                                        reveal_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.show_password).to_owned()
                                        })
                                        hide_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.hide_password).to_owned()
                                        })
                                    />
                                    <PasswordStrengthMeter
                                        score=strength
                                        strength_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.strength_label).to_owned()
                                        })
                                        level_labels=Signal::derive(move || [
                                            t_string!(i18n, onboarding.strength_weak).to_owned(),
                                            t_string!(i18n, onboarding.strength_fair).to_owned(),
                                            t_string!(i18n, onboarding.strength_good).to_owned(),
                                            t_string!(i18n, onboarding.strength_strong).to_owned(),
                                        ])
                                    />
                                    <Input
                                        id="master-password-confirm"
                                        input_type="password"
                                        placeholder=Signal::derive(move || {
                                            t_string!(i18n, onboarding.confirm_placeholder).to_owned()
                                        })
                                        value=Signal::derive(move || confirm.get())
                                        on_input=Callback::new(move |v: String| confirm.set(v))
                                        reveal_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.show_password).to_owned()
                                        })
                                        hide_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.hide_password).to_owned()
                                        })
                                    />
                                </div>
                                <div class="flex justify-between">
                                    <Button
                                        variant=Variant::Ghost
                                        on:click=move |_| current_step.set(0)
                                    >
                                        {move || t!(i18n, onboarding.back)}
                                    </Button>
                                    {move || {
                                        let enabled = password_valid() && !creating.get();
                                        view! {
                                            <Button
                                                variant=Variant::Primary
                                                disabled=!enabled
                                                attr:data-testid="onboarding-create"
                                                on:click=move |_| {
                                                    if creating.get() {
                                                        return;
                                                    }
                                                    creating.set(true);
                                                    let vault_path = ensure_vedge_home(&path.get());
                                                    let master_password = pw.get();
                                                    let msg_exists = t_string!(i18n, onboarding.err_exists)
                                                        .to_owned();
                                                    let msg_create = t_string!(i18n, onboarding.err_create)
                                                        .to_owned();
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
                                                                let dto = RecentVaultDto {
                                                                    id: Uuid::new_v4().to_string(),
                                                                    path: input.vault_path.clone(),
                                                                    display_name: display_name_from_path(&input.vault_path),
                                                                    last_opened: None,
                                                                    sort_order: 0,
                                                                };
                                                                let _ = api::recent::add_recent_vault(&dto).await;
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
                        },
                    )
                }
                _ => {
                    EitherOf3::C(
                        // ---- Step 3: Emergency Kit --------------------------------
                        view! {
                            <div class="flex flex-col gap-4">
                                <div class="flex flex-col gap-2">
                                    <span class="text-sm text-text-secondary">
                                        {move || t!(i18n, onboarding.secret_intro)}
                                    </span>
                                    <SecretDisplay value=Signal::derive(move || secret.get()) />
                                </div>
                                {move || {
                                    (!keychain_ok.get())
                                        .then(|| {
                                            view! {
                                                <p class="text-sm text-warning-text">
                                                    {move || t!(i18n, onboarding.keychain_warning)}
                                                </p>
                                            }
                                        })
                                }}
                                <div class="flex flex-wrap items-center gap-2">
                                    <Button
                                        variant=Variant::Secondary
                                        on:click=move |_| {
                                            let vault_path = path.get();
                                            let dialog_title = t_string!(
                                                i18n, onboarding.save_kit_dialog_title
                                            )
                                                .to_owned();
                                            let msg_saved = t_string!(i18n, onboarding.kit_saved)
                                                .to_owned();
                                            let msg_kit_err = t_string!(i18n, onboarding.err_kit_save)
                                                .to_owned();
                                            let msg_dialog_err = t_string!(i18n, onboarding.err_dialog)
                                                .to_owned();
                                            spawn_local(async move {
                                                let opts = SaveDialogOptions {
                                                    title: Some(dialog_title),
                                                    default_path: Some("vedge-emergency-kit.pdf".to_owned()),
                                                    filters: vec![
                                                        DialogFilter {
                                                            name: "PDF".to_owned(),
                                                            extensions: vec!["pdf".to_owned()],
                                                        },
                                                    ],
                                                };
                                                match api::dialog::save(&opts).await {
                                                    Ok(Some(dest)) => {
                                                        match api::emergency_kit::write_pdf(&vault_path, &dest)
                                                            .await
                                                        {
                                                            Ok(()) => show_success(msg_saved),
                                                            Err(e) => show_error(format!("{msg_kit_err}{e}")),
                                                        }
                                                    }
                                                    Ok(None) => {}
                                                    Err(e) => show_error(format!("{msg_dialog_err}{e}")),
                                                }
                                            });
                                        }
                                    >
                                        {move || t!(i18n, onboarding.download_kit)}
                                    </Button>
                                </div>
                                <div
                                    class="flex items-center gap-2 text-sm text-text-primary"
                                    data-testid="onboarding-ack"
                                >
                                    <Checkbox
                                        checked=Signal::derive(move || acknowledged.get())
                                        on_change=Callback::new(move |v: bool| acknowledged.set(v))
                                        aria_label=Signal::derive(move || {
                                            t_string!(i18n, onboarding.ack_aria).to_owned()
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
                                                attr:data-testid="onboarding-finish"
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
                        },
                    )
                }
            }}
        </div>
    }
}
