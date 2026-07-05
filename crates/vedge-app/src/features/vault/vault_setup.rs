//! Create-vault onboarding wizard.
//!
//! Location → master password → create → Emergency Kit (acknowledged) → land
//! unlocked at `/v/vault`. Follows the app's `spawn_local` + `use_navigate`
//! IPC idiom (see `pages/page.rs`). Copy is English literals for now — i18n
//! wiring is a follow-up.

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

use vedge_ui::components::{Button, Checkbox, Input, PasswordStrengthMeter, Step, StepIndicator};
use vedge_ui::primitives::tokens::Variant;

/// Minimum strength (0–4) required to leave the password step.
const MIN_STRENGTH: u8 = 2;

#[component]
pub fn VaultSetup() -> impl IntoView {
    let active = expect_context::<ActiveVault>();

    let current_step = RwSignal::new(0usize);
    let path = RwSignal::new(String::new());
    let pw = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    let created = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let secret = RwSignal::new(String::new());
    let keychain_ok = RwSignal::new(true);
    let acknowledged = RwSignal::new(false);
    let kit_msg = RwSignal::new(Option::<String>::None);

    let strength = Memo::new(move |_| password_score(&pw.get()));

    let can_next_location = move || !path.get().trim().is_empty();
    let password_valid = move || {
        let p = pw.get();
        !p.is_empty() && p == confirm.get() && strength.get() >= MIN_STRENGTH
    };
    let can_finish = move || created.get() && acknowledged.get();

    let steps = Signal::stored(vec![
        Step::new("Location"),
        Step::new("Password"),
        Step::new("Emergency Kit"),
    ]);

    view! {
        <div class="mx-auto flex h-full w-full max-w-xl flex-col gap-6 p-8">
            <h1 class="text-xl font-semibold text-text-primary">"Create a new vault"</h1>
            <StepIndicator steps=steps current_step=current_step />

            {move || match current_step.get() {
                // ---- Step 1: location -------------------------------------
                0 => EitherOf3::A(view! {
                    <div class="flex flex-col gap-4">
                        <div class="flex flex-col gap-1">
                            <span class="text-sm text-text-secondary">"Vault location"</span>
                            <div class="flex gap-2">
                                <Input
                                    id="vault-path"
                                    placeholder=Signal::derive(|| "…/my-vault.vdb".to_string())
                                    value=Signal::derive(move || path.get())
                                    on_input=Callback::new(move |v: String| path.set(v))
                                    class="flex-1"
                                />
                                <Button
                                    variant=Variant::Secondary
                                    on:click=move |_| {
                                        spawn_local(async move {
                                            let opts = SaveDialogOptions {
                                                title: Some("Choose vault location".to_string()),
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
                                    "Choose…"
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
                                        "Next"
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
                            <span class="text-sm text-text-secondary">"Master password"</span>
                            <Input
                                id="master-password"
                                input_type="password"
                                placeholder=Signal::derive(|| "Master password".to_string())
                                value=Signal::derive(move || pw.get())
                                on_input=Callback::new(move |v: String| pw.set(v))
                                reveal_label="Show password"
                                hide_label="Hide password"
                            />
                            <PasswordStrengthMeter
                                score=strength
                                strength_label="Password strength"
                                level_labels=Signal::stored([
                                    "Weak".to_string(),
                                    "Fair".to_string(),
                                    "Good".to_string(),
                                    "Strong".to_string(),
                                ])
                            />
                            <Input
                                id="master-password-confirm"
                                input_type="password"
                                placeholder=Signal::derive(|| "Confirm password".to_string())
                                value=Signal::derive(move || confirm.get())
                                on_input=Callback::new(move |v: String| confirm.set(v))
                                reveal_label="Show password"
                                hide_label="Hide password"
                            />
                        </div>
                        {move || error.get().map(|e| view! {
                            <p class="text-sm" style="color:var(--color-danger-text)">{e}</p>
                        })}
                        <div class="flex justify-between">
                            <Button variant=Variant::Ghost on:click=move |_| current_step.set(0)>
                                "Back"
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
                                            error.set(None);
                                            let vault_path = path.get();
                                            let master_password = pw.get();
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
                                                        error.set(Some(
                                                            "A vault already exists at this location."
                                                                .to_string(),
                                                        ));
                                                    }
                                                    Err(e) => {
                                                        error.set(Some(format!(
                                                            "Could not create the vault: {e}"
                                                        )));
                                                    }
                                                }
                                                creating.set(false);
                                            });
                                        }
                                    >
                                        "Create vault"
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
                                "Your Secret Key — save it now. It is shown only once."
                            </span>
                            <SecretDisplay value=Signal::derive(move || secret.get()) />
                        </div>
                        {move || (!keychain_ok.get()).then(|| view! {
                            <p class="text-sm" style="color:var(--color-warning-text)">
                                "This key could not be saved to your OS keychain. The Emergency Kit is your only way back into this vault — store it safely."
                            </p>
                        })}
                        <div class="flex flex-wrap items-center gap-2">
                            <Button
                                variant=Variant::Secondary
                                on:click=move |_| {
                                    kit_msg.set(None);
                                    let vault_path = path.get();
                                    spawn_local(async move {
                                        let opts = SaveDialogOptions {
                                            title: Some("Save Emergency Kit".to_string()),
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
                                                    Ok(()) => kit_msg
                                                        .set(Some("Emergency Kit saved.".to_string())),
                                                    Err(e) => kit_msg.set(Some(format!(
                                                        "Could not save the kit: {e}"
                                                    ))),
                                                }
                                            }
                                            Ok(None) => {}
                                            Err(e) => {
                                                kit_msg.set(Some(format!("Dialog failed: {e}")))
                                            }
                                        }
                                    });
                                }
                            >
                                "Download Emergency Kit (PDF)"
                            </Button>
                            {move || kit_msg.get().map(|m| view! {
                                <span class="text-sm text-text-secondary">{m}</span>
                            })}
                        </div>
                        <div class="flex items-center gap-2 text-sm text-text-primary">
                            <Checkbox
                                checked=Signal::derive(move || acknowledged.get())
                                on_change=Callback::new(move |v: bool| acknowledged.set(v))
                                aria_label="I have saved my Secret Key"
                            />
                            <span>"I have saved my Secret Key / Emergency Kit."</span>
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
                                        "Finish"
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
