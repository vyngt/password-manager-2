use crate::models::theme::color_scheme::{ColorScheme, ColorSchemeOut};
use crate::models::theme::theme::Theme;

use crate::state::AppDBConn;

#[tauri::command]
pub fn get_theme_cs(id: i64, app_db_conn: tauri::State<AppDBConn>) -> Option<ColorSchemeOut> {
    // Only color related fields
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    ColorScheme::get_color_cs(db, id)
}

#[tauri::command]
pub fn get_current_cs(app_db_conn: tauri::State<AppDBConn>) -> i64 {
    // Only color related fields
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    Theme::get_current_color_scheme(db)
}

#[tauri::command]
pub fn save_theme_cs(id: i64, app_db_conn: tauri::State<AppDBConn>) -> bool {
    let mut conns = app_db_conn.0.lock().unwrap();
    let db = &mut conns.theme_db;

    let mut current_theme = Theme::get(db).unwrap();
    current_theme.color_scheme_id = id;
    Theme::update(db, current_theme)
}
