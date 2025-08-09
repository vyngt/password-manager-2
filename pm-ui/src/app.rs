use crate::components::button::Button;
use crate::components::button::base::{ButtonEffect, ButtonShape, ButtonSize, ButtonVariant};
use crate::constants::Color;
use crate::types::color::RgbColor;
use crate::utils::color::get_css_var_color;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{HtmlElement, window};

#[component]
pub fn R1() -> impl IntoView {
    view! { <h1>"R1"</h1> }
}

#[component]
pub fn R2() -> impl IntoView {
    view! { <h1>"R2"</h1> }
}

#[component]
pub fn Home() -> impl IntoView {
    let (value, set_value) = signal(0);

    view! {
        <input
            type="color"
            on:input:target=move |ev| {
                let value = ev.target().value();
                let color = RgbColor::from_hex(&value);
                if let Some(win) = window() {
                    if let Some(doc) = win.document() {
                        if let Some(root) = doc.document_element() {
                            let html: HtmlElement = root.unchecked_into();
                            let style = html.style();
                            let _ = style
                                .set_property(
                                    "--color-primary",
                                    format!("rgb({}, {}, {})", color.r, color.g, color.b).as_str(),
                                );
                        }
                    }
                }
            }
        />
        <Button
            on:click=Box::new(move |_| {
                let color = get_css_var_color(&Color::Primary);
                let text_color = color.calculate_white_black_text_color(None);
                log!("Color: {:#?}", color);
                log!("Text color: {:#?}", text_color);
            })
            variant=ButtonVariant::Filled
            color=get_css_var_color(&Color::Primary)
            effect=ButtonEffect::Ripple
            size=ButtonSize::Small
        >
            Hello world 1
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Secondary)
            effect=ButtonEffect::Ripple
            variant=ButtonVariant::Outlined
            shape=ButtonShape::Sharp
        >
            "Hello world 2"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Success)
            effect=ButtonEffect::Ripple
            size=ButtonSize::Large
            shape=ButtonShape::Pill
        >

            "Hello world 3"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Danger)
        >
            "Hello world 4"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Warning)
        >
            "Hello world 5"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Background)
            variant=ButtonVariant::Outlined
        >
            "Hello world 6"
        </Button>
        <Button
            on:click=Box::new(move |_| set_value.update(|value| *value += 1))
            color=get_css_var_color(&Color::Foreground)
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
