#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate diesel;
extern crate diesel_migrations;

mod core;
mod db;
mod encrypt;

use crate::core::{auth, cmd, config};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn main() {
    dotenv().ok();
    config::init_config();

    let mut connection = db::establish_connection();

    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("Error migrating");

    tauri::Builder::default()
        .manage(core::state::AppState::new())
        .manage(core::state::AppDBState::new())
        .invoke_handler(tauri::generate_handler![
            cmd::create_item,
            cmd::fetch_item,
            cmd::update_item,
            cmd::delete_item,
            cmd::get_item,
            cmd::fetch_all_items,
            auth::login,
            auth::register,
            auth::is_first_time
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
