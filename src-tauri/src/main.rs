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
            cmd::item::create_item,
            cmd::item::fetch_item,
            cmd::item::update_item,
            cmd::item::delete_item,
            cmd::item::get_item,
            cmd::item::fetch_all_items,
            cmd::generator::generate_password,
            cmd::backup::export,
            cmd::backup::import,
            auth::login,
            auth::register,
            auth::is_first_time
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
