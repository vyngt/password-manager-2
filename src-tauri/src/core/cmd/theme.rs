use super::super::state::AppDBState;
use crate::db::models::color_scheme::{ColorScheme, CreateColorScheme};
use crate::db::models::theme::Theme;

#[tauri::command]
pub fn save_theme(id: i32, conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;
    let mut current_theme = Theme::get_theme(c);
    current_theme.color_scheme_id = Some(id);
    Theme::update(c, current_theme)
}

#[tauri::command]
pub fn get_current_color_scheme(conn: tauri::State<AppDBState>) -> Option<ColorScheme> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;
    let current_theme = Theme::get_theme(c);

    match current_theme.color_scheme_id {
        Some(color_sch_id) => ColorScheme::get(c, &color_sch_id),
        None => None,
    }
}

#[tauri::command]
pub fn get_all_color_schemes(conn: tauri::State<AppDBState>) -> Vec<ColorScheme> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    ColorScheme::all(c)
}

#[tauri::command]
pub fn get_color_scheme(id: i32, conn: tauri::State<AppDBState>) -> Option<ColorScheme> {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    ColorScheme::get(c, &id)
}

#[tauri::command]
pub fn create_color_scheme(data: CreateColorScheme, conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    ColorScheme::create(c, data)
}

#[tauri::command]
pub fn update_color_scheme(data: ColorScheme, conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    ColorScheme::update(c, data)
}

#[tauri::command]
pub fn delete_color_scheme(id: i32, conn: tauri::State<AppDBState>) -> bool {
    let mut c = conn.0.lock().unwrap();
    let c = &mut c.db;

    ColorScheme::delete(c, &id)
}
