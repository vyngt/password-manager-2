use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

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
        <div class="bg-background">
            <p class="text-foreground">Hello world</p>
        </div>
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
