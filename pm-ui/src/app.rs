use crate::components::button::Button;
use crate::components::button::base::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};
use crate::stores::color::{ColorStore, ColorStoreStoreFields};
use crate::types::color::RgbColor;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use reactive_stores::Store;

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
pub fn Home() -> impl IntoView {
    let (value, set_value) = signal(0);
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
            color_store.secondary().set(color);

        }
        value=move || color_store.secondary().get().to_hex()
    />


        <Button
            variant=ButtonVariant::Filled
            color=Signal::derive(move || color_store.primary().get())
            effect=ButtonEffect::Ripple
            size=ButtonSize::Small
        >
            "Hello world 1"
        </Button>
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
pub fn App() -> impl IntoView {
    let color_store = Store::new(ColorStore::new());
    provide_context(color_store);

    view! {
        <Router>
            <nav>
                <A href="/">"Home"</A>
                <A href="/r1">"R1"</A>
                <A href="/r2">"R2"</A>
            </nav>
            <main>
                <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/r1") view=R1 />
                    <Route path=path!("/r2") view=R2 />
                </Routes>
            </main>
        </Router>
    }
}
