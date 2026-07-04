//! Typed API layer over the raw Tauri `invoke()` binding.
//!
//! - `tauri` — raw `invoke()` + `TauriWindow` FFI. The single FFI boundary.
//! - `call` — generic `call<I, O>()` dispatcher. Crate-private.
//! - `error` — `ApiError` enum mirroring the shell's `CommandError`.
//! - `vault`, `entry`, `tag`, `document`, `password`, `maintenance`,
//!   `recent`, `settings`, `device`, `emergency_kit`, `recovery`,
//!   `window` — one module per command group in `vedge-tauri`.

pub mod call;
pub mod error;
pub mod tauri;

pub mod device;
pub mod dialog;
pub mod document;
pub mod emergency_kit;
pub mod entry;
pub mod maintenance;
pub mod password;
pub mod recent;
pub mod recovery;
pub mod settings;
pub mod tag;
pub mod vault;
pub mod window;

pub use error::ApiError;
