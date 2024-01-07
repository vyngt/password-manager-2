use diesel::prelude::*;
use diesel::SqliteConnection;
use std::{env, path};

pub mod schema;

use crate::config;

pub fn is_debug() -> bool {
    match env::var("DEBUG") {
        Ok(s) => {
            if s != "0" {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub fn assemble_db_url(product_db: &str, debug_var: &str, default_db: &str) -> String {
    let debug = is_debug();
    if debug {
        let debug_db_url = env::var(debug_var).unwrap_or_else(|_| default_db.to_string());

        debug_db_url
    } else {
        let db_path = path::Path::new(&tauri::api::path::home_dir().unwrap())
            .join(config::APP_DIR)
            .join(product_db);

        db_path.to_str().unwrap().to_string()
    }
}

pub fn establish_core_connection() -> SqliteConnection {
    let db_url = assemble_db_url(config::CORE_DATA, "DATABASE_CORE_URL", "core.local.db");
    SqliteConnection::establish(&db_url).expect(&format!("Error connecting to {}", db_url))
}

pub fn establish_theme_connection() -> SqliteConnection {
    let db_url = assemble_db_url(config::THEME_DATA, "DATABASE_THEME_URL", "theme.local.db");
    SqliteConnection::establish(&db_url).expect("Format connection error")
}
