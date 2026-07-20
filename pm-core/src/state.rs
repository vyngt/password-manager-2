use crate::db;
use diesel::SqliteConnection;
use std::sync::Mutex;

pub struct DBConnection {
    pub theme_db: SqliteConnection,
    pub core_db: SqliteConnection,
}

pub struct AppDBConn(pub Mutex<DBConnection>);

impl AppDBConn {
    pub fn new() -> AppDBConn {
        AppDBConn(Mutex::new(DBConnection {
            theme_db: db::establish_theme_connection(),
            core_db: db::establish_core_connection(),
        }))
    }
}
