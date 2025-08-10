use crate::stores::color::ColorStore;

use leptos::prelude::*;

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
            <main>
                <Routes fallback=|| view! { <h1>"Not Found"</h1> }>
                    <ParentRoute path=path!("/playground") view=PlayGroundLayout>
                        <Route path=path!("/") view=Playground />
                        <Route path=path!("/r1") view=R1 />
                        <Route path=path!("/r2") view=R2 />
                    </ParentRoute>
                </Routes>
            </main>
        </Router>
    }
}
