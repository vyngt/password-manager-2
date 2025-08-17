use crate::stores::color::{ColorStore, ColorStoreStoreFields};

use leptos::prelude::*;

use crate::features::window_panel::WindowPanel;
use crate::pages::page::Page as EntryPage;
use crate::pages::playground::{PlayGroundLayout, Playground, R1, R2};
use leptos_router::components::*;
use leptos_router::path;
use reactive_stores::Store;

#[component]
pub fn App() -> impl IntoView {
    let color_store = Store::new(ColorStore::new());
    provide_context(color_store);

    view! {
        <Router>
            <main class="h-full flex flex-col bg-background text-foreground"
                style=move || {
                    let primary_color = color_store.primary().get().to_rgb_string();
                    let secondary_color = color_store.secondary().get().to_rgb_string();
                    let success_color = color_store.success().get().to_rgb_string();
                    let danger_color = color_store.danger().get().to_rgb_string();
                    let warning_color = color_store.warning().get().to_rgb_string();
                    let bg_color = color_store.background().get().to_rgb_string();
                    let fg_color = color_store.foreground().get().to_rgb_string();
                    let colors = vec![
                        format!("--color-primary: {}", primary_color),
                        format!("--color-secondary: {}", secondary_color),
                        format!("--color-success: {}", success_color),
                        format!("--color-danger: {}", danger_color),
                        format!("--color-warning: {}", warning_color),
                        format!("--color-background: {}", bg_color),
                        format!("--color-foreground: {}", fg_color),
                    ];
                    colors.join(";")
                }
            >
                <WindowPanel />
                <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                    <ParentRoute path=path!("/playground") view=PlayGroundLayout>
                        <Route path=path!("/") view=Playground />
                        <Route path=path!("/r1") view=R1 />
                        <Route path=path!("/r2") view=R2 />
                    </ParentRoute>
                    <Route path=path!("/") view=EntryPage />
                </Routes>
            </main>
        </Router>
    }
}
