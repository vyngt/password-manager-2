mod app;
mod components;
mod constants;
mod features;
mod utils;

use leptos::prelude::*;

fn main() {
    leptos::mount::mount_to_body(|| view! { <app::App /> })
}
