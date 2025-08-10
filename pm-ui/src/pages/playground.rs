use crate::api::tauri::get_current_window;
use crate::components::button::Button;
use crate::components::button::variants::{
    Effect as ButtonEffect, Shape as ButtonShape, Size as ButtonSize, Variant as ButtonVariant,
};
use crate::components::icon_button::IconButton;
use crate::components::icon_button::variants::{
    Effect as IconButtonEffect, Shape as IconButtonShape, Size as IconButtonSize,
    Variant as IconButtonVariant,
};
use crate::components::input::Input;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use crate::types::color::RgbColor;
use icondata as i;
use leptos::ev::Targeted;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use leptos_router::components::*;
use reactive_stores::Store;
use web_sys::{Event, HtmlInputElement};

#[component]
pub fn R1() -> impl IntoView {
    let color_store = expect_context::<Store<ColorStore>>();

    view! {
        <div>
            <Button
                variant=ButtonVariant::Filled
                color=Signal::derive(move || color_store.primary().get())
                effect=ButtonEffect::Ripple
                size=ButtonSize::Small
            >
                "R1 Context"
            </Button>
        </div>
    }
}

#[component]
pub fn R2() -> impl IntoView {
    view! { <h1>"R2"</h1> }
}

#[component]
pub fn Playground() -> impl IntoView {
    let (value, set_value) = signal(0);
    let (text, set_text) = signal(String::new());
    let color_store: Store<ColorStore> = expect_context::<Store<ColorStore>>();

    view! {
        <input
            type="color"
            on:input:target=move |ev| {
                let value = ev.target().value();
                let color = RgbColor::from_hex(&value);
                color_store.primary().set(color);
            }
            value=move || color_store.primary().get().to_hex()
        />


        <div class="bg-violet-100 flex flex-row gap-2"
            data-tauri-drag-region=true>
            <IconButton
                color=Signal::derive(move || color_store.primary().get())
                size=IconButtonSize::Small
            >
                <Icon icon={i::FaPlusSolid}/>
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.secondary().get())
                variant=IconButtonVariant::Outlined
                size=IconButtonSize::Large
                shape=IconButtonShape::Pill
                effect=IconButtonEffect::Ripple
            >
                <Icon icon={i::FaPlusSolid}/>
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=IconButtonVariant::Text
                effect=IconButtonEffect::Ripple
                shape=IconButtonShape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move  {
                        let app_window = get_current_window();
                        app_window.minimize().await;
                    });
                }
            >
                <Icon icon={i::FaWindowMinimizeSolid}/>
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=IconButtonVariant::Text
                effect=IconButtonEffect::Ripple
                shape=IconButtonShape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move  {
                        let app_window = get_current_window();
                        let is_maximized = app_window.is_maximized().await;
                        if is_maximized.as_bool().unwrap_or(false) {
                            app_window.unmaximize().await;
                        } else {
                            app_window.maximize().await;
                        }
                    });
                }
            >
                <Icon icon={i::FaWindowMaximizeSolid}/>
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=IconButtonVariant::Text
                effect=IconButtonEffect::Ripple
                shape=IconButtonShape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move  {
                        let app_window = get_current_window();
                        app_window.close().await;
                    });
                }
            >
                <Icon icon={i::FaXmarkSolid}/>
            </IconButton>
        </div>

        <div class="w-[500px] relative">
            <Input
                id="primary"
                placeholder="Primary"
                color=Signal::derive(move || color_store.primary().get())
                input_type="text"
                on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                    let value = ev.target().value();
                    set_text.set(value);
                })
            />
            <Button
                variant=ButtonVariant::Filled
                color=Signal::derive(move || color_store.primary().get())
                effect=ButtonEffect::Ripple
                size=ButtonSize::Small
                class="absolute right-[3px] top-[3px]"
            >
                "Hello world 1"
            </Button>
        </div>

        <p>"Text: "{text}</p>

        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.secondary().get())
            effect=ButtonEffect::Ripple
            variant=ButtonVariant::Outlined
            shape=ButtonShape::Sharp
        >
            "Hello world 2"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.success().get())
            effect=ButtonEffect::Ripple
            size=ButtonSize::Large
            shape=ButtonShape::Pill
        >

            "Hello world 3"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.danger().get())
        >
            "Hello world 4"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.warning().get())
        >
            "Hello world 5"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.background().get())
            variant=ButtonVariant::Outlined
        >
            "Hello world 6"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.foreground().get())
            variant=ButtonVariant::Outlined
        >
            "Hello world 7"
        </Button>
        <div class="bg-gradient-to-tl from-blue-800 to-blue-500 text-white font-mono flex flex-col min-h-screen">
            <div class="flex flex-row-reverse flex-wrap m-auto">
                <button
                    on:click=move |_| set_value.update(|value| *value += 1)
                    class="rounded px-3 py-2 m-1 border-b-4 border-l-2 shadow-lg bg-blue-700 border-blue-800 text-white"
                >
                    "+"
                </button>
                <button class="rounded px-3 py-2 m-1 border-b-4 border-l-2 shadow-lg bg-blue-800 border-blue-900 text-white">
                    {value}
                </button>
                <button
                    on:click=move |_| set_value.update(|value| *value -= 1)
                    class="rounded px-3 py-2 m-1 border-b-4 border-l-2 shadow-lg bg-blue-700 border-blue-800 text-white"
                    class:invisible=move || { value.get() < 1 }
                >
                    "-"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn PlayGroundLayout() -> impl IntoView {
    view! {
        <div>
            <nav class="bg-gray-100 p-4">
                <A href="/playground">"Playground"</A>
                <A href="/playground/r1">"R1"</A>
                <A href="/playground/r2">"R2"</A>
            </nav>

            <Outlet />
        </div>
    }
}
