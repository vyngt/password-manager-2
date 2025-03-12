use super::v_vortex::vortex::Vortex;
use crate::db;
use diesel::SqliteConnection;
use std::{path::PathBuf, sync::Mutex};

pub struct DBConnection {
    pub theme_db: SqliteConnection,
    pub core_db: SqliteConnection,
}

pub struct AppDBConn(pub Mutex<DBConnection>);

impl AppDBConn {
    pub fn new(home_dir: &PathBuf) -> AppDBConn {
        AppDBConn(Mutex::new(DBConnection {
            theme_db: db::establish_theme_connection(home_dir),
            core_db: db::establish_core_connection(home_dir),
        }))
    }
}

pub struct AppVortex {
    pub vortex: Vortex,
}

pub struct AppVortexState(pub Mutex<AppVortex>);

impl AppVortexState {
    pub fn new(home_dir: &PathBuf) -> Self {
        let config = db::initialize_db_connections_config(home_dir);
        if let Ok(vortex) = Vortex::new(config) {
            return Self(Mutex::new(AppVortex { vortex }));
        }

        panic!("Failed to create Vortex");
    }
}
