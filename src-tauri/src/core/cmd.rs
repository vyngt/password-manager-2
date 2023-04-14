use crate::db;
use crate::db::models::item::Item;
use crate::encrypt;

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn create_item(name: &str, url: &str, username: &str, password: &str) -> bool {
    let conn = &mut db::establish_connection();
    Item::create(conn, name, url, username, password);
    return true;
}

#[tauri::command]
pub fn fetch_all_item() -> Vec<Item> {
    let conn = &mut db::establish_connection();

    let mut results = vec![];
    for item in Item::all(conn) {
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
