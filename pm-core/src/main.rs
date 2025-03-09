#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate diesel;
extern crate diesel_migrations;

mod cmd;
mod config;
mod crud;
mod db;
mod models;
mod state;
mod v_vortex;

use crate::cmd::{auth, core, password_generator, theme};

use dotenvy::dotenv;
use tauri::Manager;

fn main() {
    dotenv().ok();

    tauri::Builder::default()
        .setup(|app| {
            let home_dir = app.path().home_dir().unwrap();
            config::init_config(app);
            app.manage(state::AppDBConn::new(&home_dir));
            Ok(())
        })
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            auth::perform_auth,
            auth::rekey_auth,
            password_generator::generate_password,
            core::item::fetch_items,
            core::item::get_item,
            core::item::create_item,
            core::item::update_item,
            core::item::delete_item,
            core::item::get_item_key,
            core::item::import_vault,
            core::item::export_vault,
            theme::color_scheme::fetch_color_schemes,
            theme::color_scheme::get_color_scheme,
            theme::color_scheme::create_color_scheme,
            theme::color_scheme::update_color_scheme,
            theme::color_scheme::delete_color_scheme,
            theme::theme::get_theme_cs,
            theme::theme::get_current_cs,
            theme::theme::save_theme_cs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
