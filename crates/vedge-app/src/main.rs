mod api;
mod app;
mod features;
mod pages;
mod routes;

include!("i18n.rs");

use app::App;
use leptos::prelude::*;

fn main() {
    leptos::mount::mount_to_body(|| view! { <App /> })
}
