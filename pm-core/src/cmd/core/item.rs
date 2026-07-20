use crate::crud::define::ModelCRUD;
use crate::models::core::item::{Item, ItemCreate, ItemOut};
use crate::models::WithCount;
use crate::state::AppDBConn;
use serde_json;
use std::fs;

const LIMIT: i64 = 40;

#[tauri::command]
pub fn fetch_items(
    page: i64,
    term: &str,
    app_db_conn: tauri::State<AppDBConn>,
) -> WithCount<ItemOut> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;
    let mut _page = page;
    if _page < 1 {
        _page = 1;
    }

    Item::get_multi(db, LIMIT, (_page - 1) * LIMIT, term)
}

#[tauri::command]
pub fn create_item(data: ItemCreate<'_>, app_db_conn: tauri::State<AppDBConn>) -> Option<Item> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::create(db, data)
}

#[tauri::command]
pub fn update_item(data: Item, app_db_conn: tauri::State<AppDBConn>) -> Option<Item> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::update(db, data)
}

#[tauri::command]
pub fn delete_item(id: i64, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::delete(db, id)
}

#[tauri::command]
pub fn get_item(id: i64, app_db_conn: tauri::State<AppDBConn>) -> Option<Item> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::get(db, id)
}

#[tauri::command]
pub fn get_item_key(id: i64, app_db_conn: tauri::State<AppDBConn>) -> String {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::copy_key(db, id)
}

#[tauri::command]
pub fn export_vault(path: String, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    let results = Item::all(db);

    match serde_json::to_string(&results) {
        Ok(data) => match fs::write(path, data) {
            Ok(_) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

#[tauri::command]
pub fn import_vault(path: String, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

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

    for item in data {
        Item::create(
            db,
            ItemCreate {
                name: &item.name,
                url: &item.url,
                username: &item.username,
                password: &item.password,
            },
        );
    }

    true
}
