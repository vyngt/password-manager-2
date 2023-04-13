#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

extern crate diesel;

mod core;
mod db;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            core::greet,
            core::create_item,
            core::fetch_all_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
