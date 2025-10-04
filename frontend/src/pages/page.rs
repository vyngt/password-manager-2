use crate::api::tauri;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use leptos::ev::Targeted;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use reactive_stores::Store;
use serde_json::json;
use serde_wasm_bindgen::to_value as to_js_value;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Effect as ButtonEffect, Shape, Size, Variant};

use ui::components::DecryptIcon;
use ui::components::Input;
use ui::components::toast::provider::use_toast;
use ui::components::toast::types::ToastInput;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn Page() -> impl IntoView {
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();
    let toast = use_toast();

    let (pw, set_pw) = signal(String::new());

    let handle_submit = |value: String| {
        spawn_local(async move {
            let res = tauri::invoke(
                "unlock_vault",
                to_js_value(&json!({"request": {"key": value}})).unwrap(),
            )
            .await;

            log!("{:?}", res);
        });
    };

    return view! {
        <div class="flex h-full w-full flex-col justify-center">
            <div class="flex w-full justify-center">
                <div class="relative flex w-full max-w-[36rem]">
                    <Input
                        id="master-password"
                        placeholder="Master Password"
                        class="pr-[50px] h-[60px]"
                        label_class="text-[18px] peer-focus:text-[11px] peer-[&:not(:placeholder-shown):not(:focus)]:text-[11px]"
                        color=Signal::derive(move || color_store.primary().get())
                        input_type="password"
                        value=Signal::derive(move || pw.get())
                        on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                            set_pw.set(ev.target().value());
                        })
                        on:keydown:capture=move |ev| {
                            if ev.key() == "Enter" {
                                toast.show(ToastInput::new("Hello", None, None, color_store.secondary().get()));
                                handle_submit(pw.get());
                            }
                        }
                    />
                    <IconButton
                        color=Signal::derive(move || color_store.primary().get())
                        variant=Variant::Filled
                        size=Size::Medium
                        shape=Shape::Rounded
                        effect=ButtonEffect::Ripple
                        class="absolute right-[5px] top-[6px] p-6"
                        on:click=move |_| handle_submit(pw.get())
                    >
                        <DecryptIcon />
                    </IconButton>
                </div>
            </div>
        </div>
    };
}
