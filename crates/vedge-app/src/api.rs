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

// The per-domain command wrappers are a **complete** typed mirror of the shell's
// `#[tauri::command]` surface — a ready-made API for every command. Not everything is
// wired to UI yet, so a subset is unused. Rather than a blanket module-level allow (which
// would also hide a wrapper that *becomes* dead after a refactor), unused wrappers in a
// partially-wired module are marked `#[allow(dead_code)]` **per function**; only modules
// whose *every* wrapper is still unused carry a module-level allow (below).
pub mod biometric;
pub mod dialog;
pub mod document;
pub mod emergency_kit;
pub mod entry;
pub mod recent;
pub mod settings;
pub mod tag;
pub mod vault;
pub mod window;

// Wholly-parked subsystems — no UI calls any wrapper yet, so there are no wired functions
// whose future death a per-function allow would protect. A single module allow is honest
// here; drop it the moment the first wrapper gets wired (then switch to per-function).
#[allow(dead_code)]
pub mod device;
#[allow(dead_code)]
pub mod maintenance;
#[allow(dead_code)]
pub mod password;
#[allow(dead_code)]
pub mod recovery;
