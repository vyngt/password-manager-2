mod api;
mod app;
mod constants;
mod features;
mod pages;
mod stores;
mod utils;

use leptos::prelude::*;

fn main() {
    leptos::mount::mount_to_body(|| view! { <app::App /> })
}
