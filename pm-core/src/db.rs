use diesel::prelude::*;
use diesel::SqliteConnection;
use std::{env, path};

use crate::core::config;

pub mod actions;
pub mod models;
pub mod schema;

pub fn establish_connection() -> SqliteConnection {
    let is_debug: bool = match env::var("DEBUG") {
        Ok(s) => {
            if s != "0" {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    };

    let debug_db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "local.db".to_string());

    let mut database_url = debug_db_url.as_str();

    let db_path = path::Path::new(&tauri::api::path::home_dir().unwrap())
        .join(config::APP_DIR)
        .join(config::DATABASE);

    if !is_debug {
        database_url = db_path.to_str().clone().unwrap();
    }

    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
