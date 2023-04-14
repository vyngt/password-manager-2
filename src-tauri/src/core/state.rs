use crate::db;
use diesel::SqliteConnection;
use std::sync::Mutex;

pub struct InnerAppState {
    pub key: String,
}

impl InnerAppState {
    pub fn reset(&mut self) {
        self.key = String::from("");
    }

    pub fn set_key(&mut self, key: &str) {
        self.key = String::from(key);
    }
}

pub struct DBConnection {
    pub db: SqliteConnection,
}

pub struct AppState(pub Mutex<InnerAppState>);

impl AppState {
    pub fn new() -> AppState {
        AppState(Mutex::new(InnerAppState {
            key: String::from(""),
        }))
    }
}

pub struct AppDBState(pub Mutex<DBConnection>);

impl AppDBState {
    pub fn new() -> AppDBState {
        AppDBState(Mutex::new(DBConnection {
            db: db::establish_connection(),
        }))
    }
}
