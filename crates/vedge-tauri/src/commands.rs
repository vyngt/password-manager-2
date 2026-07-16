//! Tauri command modules. Each sub-module exposes `#[tauri::command]`
//! functions that `run()` wires into the `invoke_handler!` in Phase 10.

use std::path::Path;

use vedge_core::infrastructure::backup::target::{TargetState, read_target_state};

/// Read a vault home's plaintext `vault_uuid` without unlocking it (read-only `mode=ro`).
///
/// 🔴 Lives in the SHELL, not in an app-layer use case: `vault_uuid` is *vault* infrastructure,
/// so an app use case reaching across to open a vault would invert the dependency. Shared by
/// the recents commands (②'s duplicate check, slice 5.2.2) and the biometric commands (which
/// key the gate on the uuid since slice 5.2.3). `None` when the target is missing or unreadable.
pub(crate) async fn probe_vault_uuid(home: &Path) -> Option<String> {
    match read_target_state(home).await {
        TargetState::Readable(id) => id.vault_uuid,
        TargetState::Missing | TargetState::Unreadable => None,
    }
}

pub mod audit;
pub mod backup;
pub mod biometric;
pub mod clipboard;
pub mod device;
pub mod document;
pub mod emergency_kit;
pub mod entry;
pub mod export;
pub mod health;
pub mod maintenance;
pub mod password;
pub mod recovery;
pub mod registry;
pub mod settings;
pub mod snapshot;
pub mod tag;
pub mod totp;
pub mod vault;
