use diesel::connection::SimpleConnection;

use crate::db::run_core_migrations;
use crate::state::AppDBConn;

#[tauri::command]
pub fn perform_auth(password: &str, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let conns = &mut conns;

    let conn = &mut conns.core_db;
    conn.batch_execute(&format!("PRAGMA key={password};"))
        .unwrap();

    let result = run_core_migrations(conn);
    if result {
        match conn.batch_execute("SELECT * FROM it_work;") {
            Ok(_) => true,
            Err(_) => false,
        }
    } else {
        false
    }
}

#[tauri::command]
pub fn rekey_auth(password: &str, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let conns = &mut conns;

    let conn = &mut conns.core_db;

    match conn.batch_execute(&format!("PRAGMA rekey={password};")) {
        Ok(_) => true,
        Err(_) => false,
    }
}
