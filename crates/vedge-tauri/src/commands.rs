//! Tauri command modules. Each sub-module exposes `#[tauri::command]`
//! functions that `run()` wires into the `invoke_handler!` in Phase 10.

pub mod device;
pub mod document;
pub mod entry;
pub mod maintenance;
pub mod password;
pub mod recent;
pub mod settings;
pub mod tag;
pub mod vault;
