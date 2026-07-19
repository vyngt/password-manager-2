//! Snapshot commands (slice 5.2.1) — the TIME half of Decision ⑦.
//!
//! `create_snapshot` needs an unlocked session (it runs through the session's repo).
//! `list_snapshots` / `delete_snapshot` are 🔴 **no-session** file operations — manifests are
//! plaintext, so the launch screen can list a LOCKED or CORRUPT vault's snapshots (the H0
//! disaster path). `revert_to_snapshot` replaces the live vault, so — like `restore_vault` —
//! it refuses an *unlocked* target (Windows holds the `.vdb` open while a session is live).

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::infrastructure::backup::target::read_target_identity;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;
use vedge_core::{
    RevertToSnapshotInput, SeamlessRevertOutcome, SnapshotReason,
    create_snapshot as create_snapshot_core, revert_to_snapshot as revert_to_snapshot_core,
    revert_to_snapshot_in_session as revert_to_snapshot_in_session_core,
};

use crate::dto::snapshot::{
    RevertReportDto, SeamlessRevertDto, SnapshotDto, SnapshotReportDto, revert_report_to_dto,
    snapshot_entry_to_dto, snapshot_report_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// "Snapshot now" — capture a snapshot of the unlocked vault.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn create_snapshot(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<SnapshotReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;
    let report = create_snapshot_core(&guard, SnapshotReason::Manual).await?;
    Ok(snapshot_report_to_dto(report))
}

/// List a vault's snapshots, newest first.
///
/// 🔴 NO SESSION — reads plaintext manifests, so it works on a locked or corrupt vault (the
/// reason the store is worth building). The live vault's `verify_hash` prefix (if readable)
/// flags stale-credential snapshots.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn list_snapshots(vault_path: String) -> Result<Vec<SnapshotDto>, CommandError> {
    let home = PathBuf::from(vault_path);
    let snapshots_dir = VaultId::new(&home).snapshots_dir();
    // Best-effort: an unreadable (corrupt/locked-elsewhere) vault → no live prefix → no stale
    // badges (unknown, not alarming).
    let live_prefix = read_target_identity(&home)
        .await
        .and_then(|id| id.verify_hash_prefix);
    let entries = store::list_snapshots(&snapshots_dir).map_err(CommandError::from)?;
    Ok(entries
        .iter()
        .map(|e| snapshot_entry_to_dto(e, live_prefix.as_deref()))
        .collect())
}

/// Delete one snapshot, then sweep any objects it no longer shares. No session. 🔴 The id is
/// UNTRUSTED — `resolve_snapshot_dir` canonicalizes both sides and asserts containment.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn delete_snapshot(vault_path: String, snapshot_id: String) -> Result<(), CommandError> {
    let snapshots_dir = VaultId::new(PathBuf::from(vault_path)).snapshots_dir();
    let dir =
        store::resolve_snapshot_dir(&snapshots_dir, &snapshot_id).map_err(CommandError::from)?;
    std::fs::remove_dir_all(&dir)
        .map_err(|e| CommandError::Storage(format!("delete snapshot: {e}")))?;
    // Reclaim objects the deleted snapshot no longer shares (mark-and-sweep, best-effort).
    store::sweep_objects(&snapshots_dir).ok();
    Ok(())
}

/// Revert the vault IN PLACE to a snapshot (undoable — it auto-snapshots first). Refuses an
/// *unlocked* target; the caller locks the vault first (the launch screen is already locked).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn revert_to_snapshot(
    vault_path: String,
    snapshot_id: String,
    confirm_rollback: bool,
    state: tauri::State<'_, AppState>,
) -> Result<RevertReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(
            "lock the vault before reverting".into(),
        ));
    }
    let factory = SqliteVaultRepositoryFactory::new();
    let report = revert_to_snapshot_core(
        &factory,
        state.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: PathBuf::from(vault_path),
            snapshot_id,
            confirm_rollback,
        },
    )
    .await?;
    Ok(revert_report_to_dto(report))
}

/// In-place revert from `/v/snapshots`, starting from an **unlocked** session (the core
/// pre-flights + swaps with the session's own KEK).
///
/// 🔴 On EVERY outcome the vault ends **locked** (`stayed_unlocked: false`) and the UI navigates
/// to the launch screen to re-unlock — a revert is a destructive, credentials-affecting
/// operation, so the user re-authenticates against the reverted state rather than silently
/// continuing the pre-revert session (this supersedes slice 5.2.3's "seamless, stay-in-vault"
/// Decision ⑰). The core still rebuilds + validates the session on a healthy revert, which
/// proves the reverted snapshot opens under the current KEK before we lock. A stale snapshot
/// (re-open fails) or a rare post-teardown swap failure also locks; a re-open failure is NOT a
/// failed revert (L1). A pre-flight error leaves the session untouched (still unlocked).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn revert_to_snapshot_in_session(
    vault_path: String,
    snapshot_id: String,
    confirm_rollback: bool,
    state: tauri::State<'_, AppState>,
) -> Result<SeamlessRevertDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    // Keep the slot (get_session, not remove) so a success can replace the session in place. The
    // guard is held across the whole operation, so the reaper / screen-lock watcher / maintenance
    // cannot find and race the swap.
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let factory = SqliteVaultRepositoryFactory::new();
    let outcome = revert_to_snapshot_in_session_core(
        &guard,
        &state.unlock_vault,
        &factory,
        state.keychain.as_ref(),
        RevertToSnapshotInput {
            vault: PathBuf::from(vault_path),
            snapshot_id,
            confirm_rollback,
        },
    )
    .await;

    match outcome {
        // Pre-flight failure — the session is UNTOUCHED, so keep it: the user stays unlocked.
        Err(e) => Err(e.into()),
        Ok(SeamlessRevertOutcome::Reverted { session, report }) => {
            // 🔴 A revert is a destructive, credentials-affecting operation (it restores an older
            // state), so LOCK the vault afterward and make the user re-unlock — never silently
            // continue the pre-revert session. This is consistent with backup-restore
            // (`revert_to_snapshot` / `replace_vault`, both launch-screen paths). The core
            // rebuilt + validated the session, which PROVES the reverted snapshot opens under the
            // current KEK (so the re-unlock will succeed with the current password); we drop that
            // session and lock rather than keep it live.
            drop(session);
            drop(guard);
            let _ = state.remove_session(&vault_id);
            Ok(SeamlessRevertDto {
                stayed_unlocked: false,
                entry_count: report.entry_count,
                reverted_at: report.reverted_at,
                rollback_detected: false,
            })
        }
        Ok(SeamlessRevertOutcome::NeedsUnlock { report }) => {
            drop(guard);
            let _ = state.remove_session(&vault_id);
            Ok(SeamlessRevertDto {
                stayed_unlocked: false,
                entry_count: report.entry_count,
                reverted_at: report.reverted_at,
                rollback_detected: false,
            })
        }
        Ok(SeamlessRevertOutcome::CommitFailed { error }) => {
            drop(guard);
            let _ = state.remove_session(&vault_id);
            Err(error.into())
        }
    }
}
