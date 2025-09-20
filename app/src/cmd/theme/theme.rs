use crate::models::theme::color_scheme::{ColorScheme, ColorSchemeOut};
use crate::models::theme::theme::Theme;

use crate::state::AppVortex;

#[tauri::command]
pub fn get_theme_cs(id: i64, state: tauri::State<AppVortex>) -> Option<ColorSchemeOut> {
    // Only color related fields
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(ColorScheme::get_color_cs(conn, id))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn get_current_cs(state: tauri::State<AppVortex>) -> i64 {
    // Only color related fields
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(Theme::get_current_color_scheme(conn))) {
        Ok(result) => result,
        Err(_) => 1,
    }
}

#[tauri::command]
pub fn save_theme_cs(id: i64, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| {
        let mut current_theme = Theme::get(conn).unwrap();
        current_theme.color_scheme_id = id;
        Ok(Theme::update(conn, current_theme))
    }) {
        Ok(result) => result,
        Err(_) => false,
    }
}
