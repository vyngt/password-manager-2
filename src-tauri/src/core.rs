use crate::db;
use crate::db::models::item::Item;

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
