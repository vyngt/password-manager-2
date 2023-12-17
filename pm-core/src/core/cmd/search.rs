use crate::core::state::{AppDBState, AppState};
use crate::db::models::item::{GItem, Item};
use crate::encrypt::encryptor;

#[tauri::command]
pub fn filter_by_name(
    query: &str,
    conn: tauri::State<AppDBState>,
    profile: tauri::State<AppState>,
) -> Vec<GItem> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    let p = profile.0.lock().unwrap();
    let key = &p.key;

    let mut results = vec![];
    for item in Item::filter_by_name(c, query, 10) {
        let display_item = GItem {
            username: encryptor::decrypt_data(&key, &item.username),
            ..item
        };
        results.push(display_item)
    }

    return results;
}
