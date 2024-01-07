#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate diesel;
extern crate diesel_migrations;

// mod core;
mod config;
mod db;
// mod encrypt;

// use crate::core::{auth, cmd, config};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;

pub const CORE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/core");
pub const THEME_MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/theme");

fn main() {
    dotenv().ok();
    config::init_config();

    let mut connection = db::establish_theme_connection();
    connection
        .run_pending_migrations(THEME_MIGRATIONS)
        .expect("Error");

    // connection
    //     .run_pending_migrations(MIGRATIONS)
    //     .expect("Error migrating");

    tauri::Builder::default()
        // .manage(core::state::AppState::new())
        // .manage(core::state::AppDBState::new())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
