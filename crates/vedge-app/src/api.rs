//! Typed API layer over the raw Tauri `invoke()` binding.
//!
//! - `tauri` — raw `invoke()` + `TauriWindow` FFI. The single FFI boundary.
//! - `call` — generic `call<I, O>()` dispatcher. Crate-private.
//! - `error` — `ApiError` enum mirroring the shell's `CommandError`.
//! - `vault`, `entry`, `tag`, `document`, `password`, `recent`,
//!   `settings`, `emergency_kit`, `window` — one module per command
//!   group in `vedge-tauri`.

pub mod call;
pub mod error;
pub mod tauri;

// The per-domain command wrappers typed-mirror the shell's `#[tauri::command]` surface.
// The 2.10.1 dead-code sweep removed wrappers with no call site and no near-term consumer
// (e.g. server-side query variants superseded by client-side filtering, the bytes-based
// document/emergency-kit variants superseded by path-based ones, and the wholly-parked
// device/maintenance/recovery subsystems). A still-unused wrapper is kept only when it has a
// documented near-term consumer, marked `#[allow(dead_code, reason = "…")]` **per function**;
// a module whose *every* wrapper is still unused carries a single module-level allow (below).
pub mod audit;
pub mod biometric;
pub mod clipboard;
pub mod dialog;
pub mod document;
pub mod emergency_kit;
pub mod entry;
pub mod health;
pub mod maintenance;
pub mod recent;
pub mod settings;
pub mod tag;
pub mod totp;
pub mod vault;
pub mod window;

// Wholly-parked subsystem kept ahead of its consumer — every wrapper is still unused, so a
// single module allow is honest here; drop it the moment the first wrapper gets wired (then
// switch to per-function).
#[allow(
    dead_code,
    reason = "Phase 3 Change-Password settings flow (change_password) — no Phase 2 consumer yet"
)]
pub mod password;
