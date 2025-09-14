use diesel::connection::SimpleConnection;

use crate::db::run_core_migrations;
use crate::state::AppVortex;

#[tauri::command]
pub fn perform_auth(password: &str, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| {
        conn.batch_execute(&format!("PRAGMA key={password};"))
            .unwrap();

        let result = run_core_migrations(conn);
        if result {
            if let Ok(_) = conn.batch_execute("SELECT * FROM it_work;") {
                return Ok(true);
            }
        }
        Ok(false)
    }) {
        Ok(result) => result,
        Err(_) => false,
    }
}

#[tauri::command]
pub fn rekey_auth(password: &str, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| {
        if let Ok(_) = conn.batch_execute(&format!("PRAGMA rekey={password};")) {
            return Ok(true);
        }
        Ok(false)
    }) {
        Ok(result) => result,
        Err(_) => false,
    }
}
