mod api;
mod app;
mod constants;
mod features;
mod pages;
mod routes;
mod stores;
mod utils;

include!("i18n/mod.rs");

use app::App;
use leptos::prelude::*;

fn main() {
    leptos::mount::mount_to_body(|| view! { <App /> })
}
