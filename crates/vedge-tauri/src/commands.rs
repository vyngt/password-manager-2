//! Tauri command modules. Each sub-module exposes `#[tauri::command]`
//! functions that `run()` wires into the `invoke_handler!` in Phase 10.

pub mod audit;
pub mod biometric;
pub mod clipboard;
pub mod device;
pub mod document;
pub mod emergency_kit;
pub mod entry;
pub mod maintenance;
pub mod password;
pub mod recent;
pub mod recovery;
pub mod settings;
pub mod tag;
pub mod totp;
pub mod vault;
