use crate::api;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use vedge_ipc::UnlockVaultInputDto;
use vedge_ui::components::Tooltip;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Size, Variant};

use leptos_icons::Icon;
use vedge_ui::components::Button;
use vedge_ui::components::Input;
use vedge_ui::components::icon::Decrypt;

// TODO(UI adaptation): the unlock screen still needs a vault picker (from
// `api::recent::list_recent_vaults_with_status`) and optional recovery-kit
// input. For now this submits with an empty vault_path, which the shell
// rejects with a typed `Invalid` / `NotFound` — surfaced via
// `ApiError` in the console so it's visible what's missing next.

#[component]
pub fn Page() -> impl IntoView {
    let i18n = use_i18n();

    let (pw, set_pw) = signal(String::new());

    let handle_submit = |value: String| {
        let nav = use_navigate();
        spawn_local(async move {
            let input = UnlockVaultInputDto {
                vault_path: String::new(),
                master_password: value,
                secret_key_b64: None,
            };
            match api::vault::unlock(&input).await {
                Ok(()) => {
                    nav("/v", Default::default());
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("unlock failed: {e:?}").into());
                }
            }
        });
    };

    return view! {
        <div class="flex h-full w-full flex-col justify-center">
            <div class="flex w-full justify-center">
                <div
                    class="flex w-full max-w-xl"
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            handle_submit(pw.get());
                        }
                    }
                >
                    <Input
                        id="master-password"
                        placeholder=Signal::derive(move || {
                            t_string!(i18n, unlock.master_password).to_string()
                        })
                        size=Size::Lg
                        input_type="password"
                        value=Signal::derive(move || pw.get())
                        on_input=Callback::new(move |v: String| set_pw.set(v))
                        class="flex-1 rounded-r-none border-r-0"
                    />
                    <Tooltip content="Unlock vault">
                        <IconButton
                            aria_label="Unlock"
                            variant=Variant::Primary
                            size=Size::Lg
                            class="rounded-l-none"
                            on:click=move |_| handle_submit(pw.get())
                        >
                            <Icon icon=Decrypt />
                        </IconButton>
                    </Tooltip>
                </div>
            </div>
            <div class="mt-4 flex w-full justify-center">
                <Button
                    variant=Variant::Ghost
                    on:click=move |_| {
                        let nav = use_navigate();
                        nav("/onboarding", Default::default());
                    }
                >
                    "Create a new vault"
                </Button>
            </div>
        </div>
    };
}
