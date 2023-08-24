use crate::core::state::{AppDBState, AppState};
use crate::db::models::item::Item;
use crate::encrypt::encryptor;
use serde_json;
use std::fs;

#[tauri::command]
pub fn export(
    path: String,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let mut results = vec![];
    for item in Item::get_full(c) {
        let display_item = Item {
            username: encryptor::decrypt_data(&key, &item.username),
            password: encryptor::decrypt_data(&key, &item.password),
            ..item
        };
        results.push(display_item)
    }

    let contents = match serde_json::to_string(&results) {
        Ok(data) => data,
        Err(_) => "".into(),
    };

    match fs::write(path, contents) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[tauri::command]
pub fn import(
    path: String,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => "".into(),
    };

    let mut data: Vec<Item> = vec![];

    if *&content.len() != 0 {
        data = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => vec![],
        }
    }

    for new_item in data {
        let enc_username = encryptor::encrypt_data(&key, &new_item.username);
        let enc_password = encryptor::encrypt_data(&key, &new_item.password);
        Item::create(
            c,
            &new_item.name,
            &new_item.url,
            &enc_username,
            &enc_password,
        );
    }

    true
}
