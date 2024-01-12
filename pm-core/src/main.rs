#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate diesel;
extern crate diesel_migrations;

mod cmd;
mod config;
mod crud;
mod db;
mod models;
mod state;

use crate::cmd::{auth, core, password_generator};

use dotenvy::dotenv;

fn main() {
    dotenv().ok();
    config::init_config();

    tauri::Builder::default()
        .manage(state::AppDBConn::new())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
