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
    let name = name.trim();
    let url = url.trim();
    let username = username.trim();

    if *&url.len() == 0 && *&username.len() == 0 && *&name.len() == 0 && *&password.len() == 0
        || *&password.len() == 0
    {
        return false;
    }

    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let enc_username = encryptor::encrypt_data(&key, username);
    let enc_password = encryptor::encrypt_data(&key, password);

    Item::create(c, name, url, &enc_username, &enc_password);

    true
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
            output.url = item.url;
            output.username = encryptor::decrypt_data(&key, &item.username);
            output.password = encryptor::decrypt_data(&key, &item.password);
        }
        None => (),
    }

    output
}

#[tauri::command]
pub fn get_item(id: i32, conn: tauri::State<AppDBState>, profile: tauri::State<AppState>) -> GItem {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let mut output = GItem::default();

    let item = Item::get(c, &id);

    match item {
        Some(item) => {
            output.id = item.id;
            output.name = item.name;
            output.url = item.url;
            output.username = encryptor::decrypt_data(&key, &item.username);
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
pub fn fetch_all_items(
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> Vec<GItem> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let mut results = vec![];
    for item in Item::all(c) {
        let display_item = GItem {
            username: encryptor::decrypt_data(&key, &item.username),
            ..item
        };
        results.push(display_item)
    }

    return results;
}
