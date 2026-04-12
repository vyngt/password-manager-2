use crate::api::tauri;
use crate::i18n::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use serde_json::json;
use serde_wasm_bindgen::to_value as to_js_value;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::components::Tooltip;
use vedge_ui::primitives::tokens::{Size, Variant};

use leptos_icons::Icon;
use vedge_ui::components::Input;
use vedge_ui::components::icon::Decrypt;

#[component]
pub fn Page() -> impl IntoView {
    let i18n = use_i18n();

    let (pw, set_pw) = signal(String::new());

    let handle_submit = |value: String| {
        let nav = use_navigate();
        spawn_local(async move {
            let res = tauri::invoke(
                "unlock_vault",
                to_js_value(&json!({"request": {"key": value}})).unwrap(),
            )
            .await;

            match res.as_bool() {
                Some(res) => {
                    if res {
                        nav("/v", Default::default());
                    }
                }
                None => {}
            }
        });
    };

    return view! {
        <div class="flex h-full w-full flex-col justify-center">
            <div class="flex w-full justify-center">
                <div
                    class="flex w-full max-w-[36rem]"
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
        </div>
    };
}
