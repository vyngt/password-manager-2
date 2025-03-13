use crate::crud::define::ModelCRUD;
use crate::models::theme::color_scheme::{ColorScheme, ColorSchemeCreate};
use crate::models::WithCount;
use crate::state::AppVortex;

const LIMIT: i64 = 40;

#[tauri::command]
pub fn fetch_color_schemes(
    page: i64,
    term: &str,
    state: tauri::State<AppVortex>,
) -> WithCount<ColorScheme> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| {
        let mut _page = page;
        if _page < 1 {
            _page = 1;
        }
        let result = ColorScheme::get_multi(conn, LIMIT, (_page - 1) * LIMIT, term);
        Ok(result)
    }) {
        Ok(result) => result,
        Err(_) => WithCount {
            result: vec![],
            total: 0,
        },
    }
}

#[tauri::command]
pub fn create_color_scheme(
    data: ColorSchemeCreate<'_>,
    state: tauri::State<AppVortex>,
) -> Option<ColorScheme> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(ColorScheme::create(conn, data))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn update_color_scheme(
    data: ColorScheme,
    state: tauri::State<AppVortex>,
) -> Option<ColorScheme> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(ColorScheme::update(conn, data))) {
        Ok(result) => result,
        Err(_) => None,
    }
}

#[tauri::command]
pub fn delete_color_scheme(id: i64, state: tauri::State<AppVortex>) -> bool {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(ColorScheme::delete(conn, id))) {
        Ok(result) => result,
        Err(_) => false,
    }
}

#[tauri::command]
pub fn get_color_scheme(id: i64, state: tauri::State<AppVortex>) -> Option<ColorScheme> {
    let vortex = &state.0.lock().unwrap().vortex;
    match vortex.with_connection("theme", |conn| Ok(ColorScheme::get(conn, id))) {
        Ok(result) => result,
        Err(_) => None,
    }
}
