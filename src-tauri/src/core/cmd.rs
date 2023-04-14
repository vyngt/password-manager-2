use crate::core::state::AppDBState;
use crate::db::models::item::Item;
use crate::encrypt;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn create_item(
    name: &str,
    url: &str,
    username: &str,
    password: &str,
    conn: tauri::State<AppDBState>,
) -> bool {
    let mut _conn = conn.0.lock().unwrap();
    let _conn = &mut _conn.db;

    Item::create(_conn, name, url, username, password);
    return true;
}

#[tauri::command]
pub fn fetch_all_item(conn: tauri::State<AppDBState>) -> Vec<Item> {
    let mut _conn = conn.0.lock().unwrap();
    let _conn = &mut _conn.db;

    let mut results = vec![];
    for item in Item::all(_conn) {
        results.push(item)
    }

    return results;
}

// ----

fn get_master_password() -> String {
    let master_pw = String::from("");

    master_pw
}

#[tauri::command]
pub fn login(password: &str) -> bool {
    let password_hashed = get_master_password();
    encrypt::encryptor::check_identity(password, &password_hashed)
}
