//! Debug-only test seam (slice 5.2.3, B4) — mirrors the `resolve_keychain` gate pattern.
//!
//! The `open_old_layout_vault_and_convert` e2e needs a genuine OLD-layout vault to click Convert
//! on, but `register_vault` refuses a non-home `.vdb` path, so it cannot be seeded through the
//! normal API. This command **inverts** `migrate_vault_layout`: it takes a real `.vedge/` home
//! (built by the ordinary create flow, so it truly unlocks + decrypts) and lays it back out as a
//! legacy `<stem>.vdb` + `<stem>.vedge_blobs/`, re-pointing its recents row so the picker shows
//! **Convert**.
//!
//! 🔴 **Gated exactly like `resolve_keychain`:** the [`e2e_seed_enabled`] gate is a `cfg`-twin
//! that is compiled to a constant `false` in release, so no environment variable can talk a
//! shipped app into rearranging a user's vault files. The command stays registered but is inert.
//! The keychain entry is deliberately left untouched — the `vault_uuid` is preserved through
//! Convert, so `secret:{uuid}` keeps working and the e2e unlocks with the master password alone.

// See `commands/vault.rs` for the `#![allow]` rationale (the `#[tauri::command]` macro expands to
// `unreachable!` + non-binding `let _ =` on `#[must_use]` values).
#![allow(clippy::unreachable, clippy::let_underscore_must_use)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::repoint_registered_vault as repoint_registered_vault_core;

use crate::error::CommandError;
use crate::state::AppState;

/// Whether the e2e seeding seam may run. `cfg`-twin, exactly like `resolve_keychain`: in release
/// this is a compile-time `false`, so the command below can never rearrange vault files.
#[cfg(debug_assertions)]
fn e2e_seed_enabled() -> bool {
    std::env::var_os("VEDGE_E2E_SEED").is_some_and(|v| !v.is_empty())
}

#[cfg(not(debug_assertions))]
const fn e2e_seed_enabled() -> bool {
    false
}

fn io(op: &str, e: &std::io::Error) -> CommandError {
    CommandError::Storage(format!("{op}: {e}"))
}

/// Lay a `.vedge/` home back out as a legacy `<stem>.vdb` + `<stem>.vedge_blobs/`, and re-point
/// its recents row to the `.vdb`. Returns the legacy `.vdb` path. See the module docs.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(home_path = %home_path))]
pub async fn e2e_downgrade_to_legacy(
    home_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    if !e2e_seed_enabled() {
        return Err(CommandError::Invalid(
            "the e2e seed seam is disabled (debug + VEDGE_E2E_SEED only)".into(),
        ));
    }
    let home = PathBuf::from(&home_path);
    if state.is_unlocked(&VaultId::new(home.clone())) {
        return Err(CommandError::Invalid(
            "lock the vault before downgrading it".into(),
        ));
    }
    let parent = home
        .parent()
        .ok_or_else(|| CommandError::Invalid("home has no parent".into()))?
        .to_owned();
    let stem = home
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::Invalid("home has no file stem".into()))?
        .to_owned();
    let legacy_vdb = parent.join(format!("{stem}.vdb"));
    let legacy_blobs = parent.join(format!("{stem}.vedge_blobs"));

    // Move the DB (+ any -wal/-shm) out to the legacy path.
    std::fs::rename(home.join("vault.vdb"), &legacy_vdb).map_err(|e| io("move vault.vdb", &e))?;
    for suffix in ["-wal", "-shm"] {
        let src = home.join(format!("vault.vdb{suffix}"));
        if src.exists() {
            std::fs::rename(&src, parent.join(format!("{stem}.vdb{suffix}"))).ok();
        }
    }
    // Move blobs to the legacy sibling directory.
    if home.join("blobs").is_dir() {
        std::fs::rename(home.join("blobs"), &legacy_blobs).map_err(|e| io("move blobs", &e))?;
    }
    // Drop what remains of the home (the now-empty `snapshots/` etc.).
    std::fs::remove_dir_all(&home).ok();

    // Re-point the recents row to the legacy `.vdb` so the picker renders it as convertible.
    let rows = state
        .vault_registry
        .list()
        .await
        .map_err(CommandError::from)?;
    let row = rows
        .iter()
        .find(|r| r.path == home)
        .ok_or_else(|| CommandError::Invalid("no recents row points at this home".into()))?;
    repoint_registered_vault_core(&*state.vault_registry, &row.id, legacy_vdb.clone())
        .await
        .map_err(CommandError::from)?;

    Ok(legacy_vdb.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::e2e_seed_enabled;

    /// 🔴 The seam can NEVER be enabled in a release build — no env var reaches it, because the
    /// gate is compiled to a constant `false`. Mirrors the keychain-redirect release test.
    #[test]
    fn the_seed_seam_is_compiled_out_of_release() {
        // In a debug build the gate is env-driven, and the host suite runs without the var set.
        #[cfg(debug_assertions)]
        {
            assert!(
                std::env::var_os("VEDGE_E2E_SEED").is_none(),
                "the host test suite must not run with the e2e seed seam enabled"
            );
            assert!(!e2e_seed_enabled(), "off without the env var");
        }
        // In release, no env var can enable it — the gate is a compile-time `false`.
        #[cfg(not(debug_assertions))]
        {
            unsafe { std::env::set_var("VEDGE_E2E_SEED", "1") };
            assert!(
                !e2e_seed_enabled(),
                "the e2e seed seam must be impossible to enable in release"
            );
            unsafe { std::env::remove_var("VEDGE_E2E_SEED") };
        }
    }
}
