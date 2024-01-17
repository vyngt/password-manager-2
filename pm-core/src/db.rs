use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::SqliteConnection;
use std::{env, path};

pub mod schema;

use crate::config;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const CORE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/core");
pub const THEME_MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/theme");

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

pub fn assemble_db_url(product_db: &str, develop_db: &str) -> String {
    let debug = is_debug();
    if debug {
        let debug_db_url = develop_db.to_string();

        debug_db_url
    } else {
        let db_path = path::Path::new(&tauri::api::path::home_dir().unwrap())
            .join(config::APP_DIR)
            .join(product_db);

        db_path.to_str().unwrap().to_string()
    }
}

pub fn run_core_migrations(conn: &mut SqliteConnection) -> bool {
    let r = conn.run_pending_migrations(CORE_MIGRATIONS);
    match r {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn establish_core_connection() -> SqliteConnection {
    let db_url = assemble_db_url(config::CORE_DATA, "./local/core_local.db");
    let mut conn =
        SqliteConnection::establish(&db_url).expect(&format!("Error connecting to {}", db_url));
    conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
    conn
}

pub fn establish_theme_connection() -> SqliteConnection {
    let db_url = assemble_db_url(config::THEME_DATA, "./local/theme_local.db");
    let mut conn = SqliteConnection::establish(&db_url).expect("Format connection error");
    conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
    conn.run_pending_migrations(THEME_MIGRATIONS)
        .expect("Something terrible happen: Theme Migrations");
    conn
}
