#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate diesel;
extern crate diesel_migrations;

mod cmd;
mod config;
mod crud;
mod db;
mod models;
mod state;

use crate::cmd::auth;

use dotenvy::dotenv;

fn main() {
    dotenv().ok();
    config::init_config();

    tauri::Builder::default()
        .manage(state::AppDBConn::new())
        .invoke_handler(tauri::generate_handler![
            auth::perform_auth,
            auth::rekey_auth
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
