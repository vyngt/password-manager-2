use crate::api::tauri::get_current_window;
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use icondata as i;
use leptos::ev::Targeted;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::Icon;
use leptos_router::{MatchNestedRoutes, components::*, path};
use reactive_stores::Store;
use ui::components::button::Button;
use ui::components::icon_button::IconButton;
use ui::components::input::Input;
use ui::components::{Tooltip, TooltipPosition};
use ui::primitives::color::RgbColor;
use ui::primitives::tokens::{Effect as ButtonEffect, Shape, Size, Variant};
use web_sys::{Event, HtmlInputElement};

#[component]
fn R1() -> impl IntoView {
    let color_store = expect_context::<Store<ColorStore>>();

    view! {
        <div>
            <Button
                variant=Variant::Filled
                color=Signal::derive(move || color_store.primary().get())
                effect=ButtonEffect::Ripple
                size=Size::Small
            >
                "R1 Context"
            </Button>
        </div>
    }
}

#[component]
fn R2() -> impl IntoView {
    view! { <h1>"R2"</h1> }
}

#[component]
fn Playground() -> impl IntoView {
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

        <input
            type="color"
            on:input:target=move |ev| {
                let value = ev.target().value();
                let color = RgbColor::from_hex(&value);
                color_store.background().set(color);
            }
            value=move || color_store.background().get().to_hex()
        />

        <div class="bg-violet-100 flex flex-row gap-2" data-tauri-drag-region=true>
            <IconButton color=Signal::derive(move || color_store.primary().get()) size=Size::Small>
                <Icon icon=i::FaPlusSolid />
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.secondary().get())
                variant=Variant::Outlined
                size=Size::Large
                shape=Shape::Pill
                effect=ButtonEffect::Ripple
            >
                <Icon icon=i::FaPlusSolid />
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=Variant::Text
                effect=ButtonEffect::Ripple
                shape=Shape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move {
                        let app_window = get_current_window();
                        app_window.minimize().await;
                    });
                }
            >
                <Icon icon=i::FaWindowMinimizeSolid />
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=Variant::Text
                effect=ButtonEffect::Ripple
                shape=Shape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move {
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
                <Icon icon=i::FaWindowMaximizeSolid />
            </IconButton>
            <IconButton
                color=Signal::derive(move || color_store.danger().get())
                variant=Variant::Text
                effect=ButtonEffect::Ripple
                shape=Shape::Sharp
                on:click=move |_ev| {
                    spawn_local(async move {
                        let app_window = get_current_window();
                        app_window.close().await;
                    });
                }
            >
                <Icon icon=i::FaXmarkSolid />
            </IconButton>
        </div>

        <div class="w-[500px] relative">
            <Input
                id="primary"
                placeholder=Signal::derive(|| "Primary".to_string())
                color=Signal::derive(move || color_store.primary().get())
                input_type="text"
                on_input_target=Callback::new(move |ev: Targeted<Event, HtmlInputElement>| {
                    let value = ev.target().value();
                    set_text.set(value);
                })
            />
            <Button
                variant=Variant::Filled
                color=Signal::derive(move || color_store.primary().get())
                effect=ButtonEffect::Ripple
                size=Size::Small
                class="absolute right-[3px] top-[3px]"
            >
                "Hello world 1"
            </Button>
        </div>

        <p>"Text: "{text}</p>

        <div class="relative z-0
        h-40 p-4 text-white
        bg-black
        before:content-[''] before:absolute before:inset-0
        before:bg-purple-500 before:opacity-30 before:-z-10 before:pointer-events-none">Hello</div>

        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.secondary().get())
            effect=ButtonEffect::Ripple
            variant=Variant::Outlined
            shape=Shape::Sharp
        >
            "Hello world 2"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=Signal::derive(move || color_store.success().get())
            effect=ButtonEffect::Ripple
            size=Size::Large
            shape=Shape::Pill
        >
            "Hello world 3"
        </Button>

        <Tooltip
            position=TooltipPosition::Bottom
            class="bg-amber-600 text-white whitespace-nowrap"
            arrow=true
            content=move || view! { <div class="p-2">Hello world</div> }
            trigger=move || {
                view! {
                    <Button
                        on:click=Box::new(move |_| set_value.update(|value| *value += 1))
                        color=Signal::derive(move || color_store.danger().get())
                        effect=ButtonEffect::Ripple
                        variant=Variant::Outlined
                        shape=Shape::Sharp
                    >
                        "Hello world 4"
                    </Button>
                }
            }
        />

        <Tooltip
            position=TooltipPosition::Top
            class="bg-amber-600 text-white"
            arrow=true
            content=move || view! { <div class="p-2">Hello world</div> }
            trigger=move || {
                view! {
                    <Button
                        on:click=Box::new(move |_| set_value.update(|value| *value += 1))
                        color=Signal::derive(move || color_store.danger().get())
                        effect=ButtonEffect::Ripple
                        variant=Variant::Outlined
                        shape=Shape::Sharp
                    >
                        "Hello world 4"
                    </Button>
                }
            }
        />

        <Tooltip
            position=TooltipPosition::Left
            class="bg-amber-600 text-white"
            arrow=true
            content=move || view! { <div class="p-2">Hello world</div> }
            trigger=move || {
                view! {
                    <Button
                        on:click=Box::new(move |_| set_value.update(|value| *value += 1))
                        color=Signal::derive(move || color_store.danger().get())
                        effect=ButtonEffect::Ripple
                        variant=Variant::Outlined
                        shape=Shape::Sharp
                    >
                        "Hello world 4"
                    </Button>
                }
            }
        />

        <Tooltip
            position=TooltipPosition::Right
            class="bg-amber-600 text-white"
            arrow=true
            content=move || view! { <div class="p-2">Hello world</div> }
            trigger=move || {
                view! {
                    <Button
                        on:click=Box::new(move |_| set_value.update(|value| *value += 1))
                        color=Signal::derive(move || color_store.danger().get())
                        effect=ButtonEffect::Ripple
                        variant=Variant::Outlined
                        shape=Shape::Sharp
                    >
                        "Hello world 4"
                    </Button>
                }
            }
        />

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
fn PlayGroundLayout() -> impl IntoView {
    view! {
        <div>
            <nav class="p-4">
                <A href="/playground">"Playground"</A>
                <A href="/playground/r1">"R1"</A>
                <A href="/playground/r2">"R2"</A>
            </nav>

            <Outlet />
        </div>
    }
}

#[component(transparent)]
pub fn PlayGroundRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/playground") view=PlayGroundLayout>
            <Route path=path!("/") view=Playground />
            <Route path=path!("/r1") view=R1 />
            <Route path=path!("/r2") view=R2 />
        </ParentRoute>
    }
    .into_inner()
}
