use crate::crud::define::ModelCRUD;
use crate::models::theme::color_scheme::{ColorScheme, ColorSchemeCreate, ColorSchemeOut};
use crate::models::WithCount;
use crate::state::AppDBConn;

const LIMIT: i64 = 40;

#[tauri::command]
pub fn fetch_color_schemes(
    page: i64,
    term: &str,
    app_db_conn: tauri::State<AppDBConn>,
) -> WithCount<ColorScheme> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;
    let mut _page = page;
    if _page < 1 {
        _page = 1;
    }

    ColorScheme::get_multi(db, LIMIT, (_page - 1) * LIMIT, term)
}

#[tauri::command]
pub fn create_color_scheme(
    data: ColorSchemeCreate<'_>,
    app_db_conn: tauri::State<AppDBConn>,
) -> Option<ColorScheme> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::create(db, data)
}

#[tauri::command]
pub fn update_color_scheme(
    data: ColorScheme,
    app_db_conn: tauri::State<AppDBConn>,
) -> Option<ColorScheme> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::update(db, data)
}

#[tauri::command]
pub fn delete_color_scheme(id: i64, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::delete(db, id)
}

#[tauri::command]
pub fn get_color_scheme(id: i64, app_db_conn: tauri::State<AppDBConn>) -> Option<ColorScheme> {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::get(db, id)
}

#[tauri::command]
pub fn get_theme_cs(id: i64, app_db_conn: tauri::State<AppDBConn>) -> Option<ColorSchemeOut> {
    // Only color related fields
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::get_color_cs(db, id)
}
