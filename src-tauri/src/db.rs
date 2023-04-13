use diesel::prelude::*;
use diesel::SqliteConnection;

pub mod actions;
pub mod models;
pub mod schema;

pub fn establish_connection() -> SqliteConnection {
    let database_url = "./vault.db";
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
