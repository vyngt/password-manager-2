use crate::crud::define::ModelCRUD;
use crate::models::core::item::{Item, ItemCreate, ItemOut};
use crate::models::WithCount;
use crate::state::AppVortex;
use serde_json;
use std::fs;

const LIMIT: i64 = 40;

#[tauri::command]
pub fn fetch_items(page: i64, term: &str, state: tauri::State<AppVortex>) -> WithCount<ItemOut> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| {
        let mut _page = page;
        if _page < 1 {
            _page = 1;
        }
        Ok(Item::get_multi(conn, LIMIT, (_page - 1) * LIMIT, term))
    }) {
        Ok(result) => result,
        Err(_) => WithCount {
            result: vec![],
            total: 0,
        },
    }
}

#[tauri::command]
pub fn create_item(data: ItemCreate<'_>, state: tauri::State<AppVortex>) -> Option<Item> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| Ok(Item::create(conn, data))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn update_item(data: Item, state: tauri::State<AppVortex>) -> Option<Item> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| Ok(Item::update(conn, data))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn delete_item(id: i64, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| Ok(Item::delete(conn, id))) {
        Ok(result) => result,
        Err(_) => false,
    }
}

#[tauri::command]
pub fn get_item(id: i64, state: tauri::State<AppVortex>) -> Option<Item> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| Ok(Item::get(conn, id))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn get_item_key(id: i64, state: tauri::State<AppVortex>) -> String {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| Ok(Item::copy_key(conn, id))) {
        Ok(result) => result,
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub fn export_vault(path: String, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("core", |conn| {
        let results = Item::all(conn);
        match serde_json::to_string(&results) {
            Ok(data) => match fs::write(path, data) {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
            Err(_) => Ok(false),
        }
    }) {
        Ok(result) => result,
        Err(_) => false,
    }
}

#[tauri::command]
pub fn import_vault(path: String, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
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

    match vortex.with_connection("core", |conn| {
        for item in data {
            Item::create(
                conn,
                ItemCreate {
                    name: &item.name,
                    url: &item.url,
                    username: &item.username,
                    password: &item.password,
                },
            );
        }
        Ok(true)
    }) {
        Ok(result) => result,
        Err(_) => false,
    }
}
