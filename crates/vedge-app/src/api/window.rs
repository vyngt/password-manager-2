//! Window controls. Not a `#[tauri::command]` — uses Tauri's
//! `@tauri-apps/api/window` plugin directly. Re-exports the raw
//! `TauriWindow` type from `api::tauri` and adds a few ergonomic async
//! wrappers that return plain `bool` / `()`.

use wasm_bindgen::JsValue;

use crate::api::tauri::{TauriWindow, get_current_window};

#[must_use]
pub fn current() -> TauriWindow {
    get_current_window()
}

pub async fn is_maximized(window: &TauriWindow) -> bool {
    let v: JsValue = window.is_maximized().await;
    v.as_bool().unwrap_or(false)
}

pub async fn maximize(window: &TauriWindow) {
    window.maximize().await;
}

pub async fn unmaximize(window: &TauriWindow) {
    window.unmaximize().await;
}

// pub async fn toggle_maximize(window: &TauriWindow) {
//     if is_maximized(window).await {
//         window.unmaximize().await;
//     } else {
//         window.maximize().await;
//     }
// }

pub async fn minimize(window: &TauriWindow) {
    window.minimize().await;
}

pub async fn close(window: &TauriWindow) {
    window.close().await;
}

// pub async fn start_dragging(window: &TauriWindow) {
//     window.start_dragging().await;
// }
