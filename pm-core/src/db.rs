use super::state::AppVortexState;
use super::v_vortex::core::database::ConnectionConfig;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::SqliteConnection;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use tauri::Manager;

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

pub fn assemble_db_url(home_dir: &PathBuf, product_db: &str, develop_db: &str) -> String {
    let debug = is_debug();
    if debug {
        let debug_db_url = develop_db.to_string();

        debug_db_url
    } else {
        let db_path = home_dir.join(config::APP_DIR).join(product_db);

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

pub fn establish_core_connection(home_dir: &PathBuf) -> SqliteConnection {
    let db_url = assemble_db_url(home_dir, config::CORE_DATA, "./local/core_local.db");
    let mut conn =
        SqliteConnection::establish(&db_url).expect(&format!("Error connecting to {}", db_url));
    conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
    conn
}

pub fn establish_theme_connection(home_dir: &PathBuf) -> SqliteConnection {
    let db_url = assemble_db_url(home_dir, config::THEME_DATA, "./local/theme_local.db");
    let mut conn = SqliteConnection::establish(&db_url).expect("Format connection error");
    conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
    conn.run_pending_migrations(THEME_MIGRATIONS)
        .expect("Something terrible happen: Theme Migrations");
    conn
}

pub fn initialize_db_connections_config(home_dir: &PathBuf) -> HashMap<String, ConnectionConfig> {
    let mut conn_map = HashMap::new();
    conn_map.insert(
        "core".to_string(),
        ConnectionConfig::Single(assemble_db_url(
            home_dir,
            config::CORE_DATA,
            "./local/core_local.db",
        )),
    );
    conn_map.insert(
        "theme".to_string(),
        ConnectionConfig::Single(assemble_db_url(
            home_dir,
            config::THEME_DATA,
            "./local/theme_local.db",
        )),
    );

    conn_map
}

pub fn run_unencrypt_migrations(app: &mut tauri::App) {
    let state = app.state::<AppVortexState>();
    let vortex = &state.0.lock().unwrap().vortex;
    let _ = vortex.with_connection("theme", |conn| {
        conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
        conn.run_pending_migrations(THEME_MIGRATIONS)
            .expect("Something terrible happen: Theme Migrations");

        return Ok(());
    });

    return ();
}
