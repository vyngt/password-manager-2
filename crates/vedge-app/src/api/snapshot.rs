//! Snapshot commands (slice 5.2.1). Mirrors `vedge-tauri/src/commands/snapshot.rs`.
//!
//! `create` needs an unlocked session; `list` / `delete` are no-session file ops (so the
//! launch screen can list a locked or corrupt vault's snapshots — H0); `revert` refuses an
//! unlocked target (the caller locks first).

use serde::Serialize;

use vedge_ipc::{
    ImportPreviewRow, RevertReportDto, SeamlessRevertDto, SnapshotDto, SnapshotReportDto,
};

use crate::api::call::call;
use crate::api::error::ApiError;

#[derive(Serialize)]
struct VaultArg<'a> {
    vault_path: &'a str,
}

/// "Snapshot now" on the unlocked vault.
pub async fn create(vault_path: &str) -> Result<SnapshotReportDto, ApiError> {
    call("create_snapshot", &VaultArg { vault_path }).await
}

/// List the vault's snapshots, newest first (no session — works on a corrupt/locked vault).
pub async fn list(vault_path: &str) -> Result<Vec<SnapshotDto>, ApiError> {
    call("list_snapshots", &VaultArg { vault_path }).await
}

/// Delete one snapshot by id.
pub async fn delete(vault_path: &str, snapshot_id: &str) -> Result<(), ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        snapshot_id: &'a str,
    }
    call(
        "delete_snapshot",
        &Args {
            vault_path,
            snapshot_id,
        },
    )
    .await
}

/// Begin recovering entries FROM a snapshot into the live vault — the tweezers (slice 5.3c).
/// Opens the snapshot with the current session and returns the derivatives-only preview; pick
/// rows, then finish with `api::import::commit` or discard with `api::import::cancel`. A stale
/// snapshot returns an error whose message is the honest "recover with its original password"
/// copy (the launch-screen revert path is the way to open one under its old credentials).
pub async fn begin_recover(
    vault_path: &str,
    snapshot_id: &str,
) -> Result<Vec<ImportPreviewRow>, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        snapshot_id: &'a str,
    }
    call(
        "begin_snapshot_import",
        &Args {
            vault_path,
            snapshot_id,
        },
    )
    .await
}

/// Revert the vault to a snapshot IN PLACE. The target must be **locked**; `confirm_rollback`
/// is the user's explicit yes (reverting is a rollback by design — it auto-snapshots first).
pub async fn revert(
    vault_path: &str,
    snapshot_id: &str,
    confirm_rollback: bool,
) -> Result<RevertReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        snapshot_id: &'a str,
        confirm_rollback: bool,
    }
    call(
        "revert_to_snapshot",
        &Args {
            vault_path,
            snapshot_id,
            confirm_rollback,
        },
    )
    .await
}

/// Seamless in-place revert from `/v/snapshots` (slice 5.2.3, Decision ⑰) — keeps the user IN
/// the vault. Requires an **unlocked** session. `stayed_unlocked == false` in the returned DTO
/// means the vault could not be re-opened (stale snapshot) and the UI should navigate to the
/// launch screen — it is **not** a revert failure.
pub async fn revert_in_session(
    vault_path: &str,
    snapshot_id: &str,
    confirm_rollback: bool,
) -> Result<SeamlessRevertDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        snapshot_id: &'a str,
        confirm_rollback: bool,
    }
    call(
        "revert_to_snapshot_in_session",
        &Args {
            vault_path,
            snapshot_id,
            confirm_rollback,
        },
    )
    .await
}
