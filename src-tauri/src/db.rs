use diesel::prelude::*;
use diesel::SqliteConnection;
use std::path;

use crate::core::config;

pub mod actions;
pub mod models;
pub mod schema;

pub fn establish_connection() -> SqliteConnection {
    let database_url = path::Path::new(&tauri::api::path::home_dir().unwrap())
        .join(config::APP_DIR)
        .join("vault.db");

    let database_url = database_url.to_str().clone().unwrap();
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
