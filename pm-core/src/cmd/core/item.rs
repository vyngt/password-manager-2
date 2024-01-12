use crate::crud::define::ModelCRUD;
use crate::models::core::item::{Item, ItemCreate, ItemOut};
use crate::state::AppDBConn;

#[tauri::command]
pub fn fetch_items(app_db_conn: tauri::State<AppDBConn>) -> Vec<ItemOut> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::get_multi(db, 100, 0)
}

#[tauri::command]
pub fn query_items(query: &str, app_db_conn: tauri::State<AppDBConn>) -> Vec<ItemOut> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::filter_by_name(db, query, 100, 0)
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
pub fn delete_item(id: i32, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::delete(db, id)
}

#[tauri::command]
pub fn get_item(id: i32, app_db_conn: tauri::State<AppDBConn>) -> Option<Item> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::get(db, id)
}

#[tauri::command]
pub fn get_item_key(id: i32, app_db_conn: tauri::State<AppDBConn>) -> String {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.core_db;

    Item::copy_key(db, id)
}
