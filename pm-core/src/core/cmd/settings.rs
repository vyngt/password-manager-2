use crate::core::state::{AppDBState, AppState};
use crate::db::models::item::Item;
use crate::encrypt::encryptor;

use super::super::auth::set_master_password;

#[tauri::command]
pub fn change_master_password(
    password: &str,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;
    let mut state = profile.0.lock().unwrap();
    let state = &mut state;

    let previous_key = &state.key;

    let mut previous_data = vec![];
    for item in Item::get_full(c) {
        let display_item = Item {
            username: encryptor::decrypt_data(&previous_key, &item.username),
            password: encryptor::decrypt_data(&previous_key, &item.password),
            ..item
        };
        previous_data.push(display_item)
    }

    let ok = set_master_password(c, password);

    if ok {
        state.set_key(password);
    } else {
        return false;
    }

    let key = &state.key;

    for item in previous_data {
        Item::update(
            c,
            &item.id,
            &item.name,
            &item.url,
            &encryptor::encrypt_data(&key, &item.username),
            &encryptor::encrypt_data(&key, &item.password),
        );
    }

    true
}
