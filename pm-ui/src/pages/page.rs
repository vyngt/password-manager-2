use crate::components::icon_button::IconButton;
use crate::components::icon_button::variants::{
    Effect as IconButtonEffect, Shape as IconButtonShape, Size as IconButtonSize,
    Variant as IconButtonVariant,
};
use crate::components::input::Input;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use icondata as i;
use leptos::ev::Targeted;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_icons::Icon;
use reactive_stores::Store;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn Page() -> impl IntoView {
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    let (pw, set_pw) = signal(String::new());

    return view! {
        <div class="flex h-full w-full flex-col justify-center">
            <div class="flex w-full justify-center">
                <div class="relative flex w-full max-w-[24rem]">
                    <Input
                        id="master-password"
                        placeholder="Master Password"
                        class="pr-[40px]"
                        color=Signal::derive(move || color_store.primary().get())
                        input_type="password"
                        value=Signal::derive(move || pw.get())
                        on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                            set_pw.set(ev.target().value());
                        })
                        on:keydown:capture=move |ev| {
                            if ev.key() == "Enter" {
                                log!("{}", pw.get());
                            }
                        }
                    />
                    <IconButton
                        color=Signal::derive(move || color_store.primary().get())
                        variant=IconButtonVariant::Filled
                        size=IconButtonSize::Medium
                        shape=IconButtonShape::Rounded
                        effect=IconButtonEffect::Ripple
                        class="absolute right-[3px] top-[5px]"
                        on:click=move |_| log!("{}", pw.get())
                    >
                        <Icon icon=i::FaArrowRightSolid />
                    </IconButton>
                </div>
            </div>
        </div>
    };
}
