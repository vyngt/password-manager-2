use crate::core::state::{AppDBState, AppState};
use crate::db::models::item::{GItem, Item};
use crate::encrypt::encryptor;

#[tauri::command]
pub fn create_item(
    name: &str,
    url: &str,
    username: &str,
    password: &str,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let enc_username = encryptor::encrypt_data(&key, username);
    let enc_password = encryptor::encrypt_data(&key, password);

    Item::create(c, name, url, &enc_username, &enc_password);
    return true;
}

#[tauri::command]
pub fn fetch_item(
    id: i32,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> Item {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let mut output = Item::default();

    let item = Item::get(c, &id);

    match item {
        Some(item) => {
            output.id = item.id;
            output.name = item.name;
            output.username = encryptor::decrypt_data(&key, &item.username);
            output.password = encryptor::decrypt_data(&key, &item.password);
        }
        None => (),
    }

    output
}

#[tauri::command]
pub fn update_item(
    id: i32,
    name: &str,
    url: &str,
    username: &str,
    password: &str,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;
    let p = profile.0.lock().unwrap();
    let key = &p.key;

    Item::update(
        c,
        &id,
        name,
        url,
        &encryptor::encrypt_data(&key, &username),
        &encryptor::encrypt_data(&key, &password),
    )
}

#[tauri::command]
pub fn delete_item(id: i32, conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    Item::delete(c, &id)
}

#[tauri::command]
pub fn fetch_all_items(conn: tauri::State<AppDBState>) -> Vec<GItem> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let mut results = vec![];
    for item in Item::all(c) {
        results.push(item)
    }

    return results;
}
