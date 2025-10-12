use crate::api::tauri;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use leptos::ev::Targeted;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use reactive_stores::Store;
use serde_json::json;
use serde_wasm_bindgen::to_value as to_js_value;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Effect as ButtonEffect, Shape, Size, Variant};

use leptos_icons::Icon;
use ui::components::Input;
use ui::components::icon::Decrypt;

use web_sys::{Event, HtmlInputElement};

#[component]
pub fn Page() -> impl IntoView {
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

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
                        nav("/playground", Default::default());
                    }
                }
                None => {}
            }
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
                        class="absolute right-[5px] top-[6px] p-6 [&_svg]:text-[30px]"
                        on:click=move |_| handle_submit(pw.get())
                    >
                        <Icon icon=Decrypt />
                    </IconButton>
                </div>
            </div>
        </div>
    };
}
