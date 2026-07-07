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

// The per-domain command wrappers below are a **complete** typed mirror of the
// shell's `#[tauri::command]` surface. The store-data UI (Phase 1) consumes only
// a subset so far, so the not-yet-used wrappers are kept and `dead_code`-allowed
// rather than deleted — they're the ready-made API for later phases.
#[allow(dead_code)]
pub mod biometric;
#[allow(dead_code)]
pub mod device;
pub mod dialog;
#[allow(dead_code)]
pub mod document;
#[allow(dead_code)]
pub mod emergency_kit;
#[allow(dead_code)]
pub mod entry;
#[allow(dead_code)]
pub mod maintenance;
#[allow(dead_code)]
pub mod password;
#[allow(dead_code)]
pub mod recent;
#[allow(dead_code)]
pub mod recovery;
#[allow(dead_code)]
pub mod settings;
#[allow(dead_code)]
pub mod tag;
#[allow(dead_code)]
pub mod vault;
pub mod window;
