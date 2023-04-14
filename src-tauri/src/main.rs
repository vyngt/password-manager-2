#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

extern crate diesel;
extern crate diesel_migrations;

mod core;
mod db;
mod encrypt;

use crate::core::{cmd, config};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn main() {
    config::init_config();

    let mut connection = db::establish_connection();

    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("Error migrating");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd::greet,
            cmd::create_item,
            cmd::fetch_all_item,
            cmd::login
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
