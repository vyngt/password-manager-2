//! Backup + restore commands (slice 5.2 / 5.2b).
//!
//! `backup_vault` needs an unlocked session (the snapshot runs through its repo);
//! the guard is held across the archive write (see `commands/maintenance.rs` for
//! the `#![allow]` rationale). `restore_vault`/`inspect_backup` are the inverse —
//! **file** operations with no session: restore refuses an *unlocked* target and
//! inspect is a read-only preview.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;
use vedge_core::{
    BackupVaultInput, InspectBackupInput, MigrateVaultLayoutInput, RestoreVaultInput,
    backup_vault as backup_vault_core, inspect_backup as inspect_backup_core,
    migrate_vault_layout as migrate_vault_layout_core, restore_vault as restore_vault_core,
};

use crate::dto::backup::{
    BackupPreviewDto, BackupReportDto, RestoreReportDto, backup_preview_to_dto,
    backup_report_to_dto, restore_report_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn backup_vault(
    vault_path: String,
    dest_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let report = backup_vault_core(
        &guard,
        BackupVaultInput {
            dest_archive: PathBuf::from(dest_path),
        },
    )
    .await?;
    Ok(backup_report_to_dto(report))
}

/// Preview a `.vbk` against a target vault (read-only, no session).
///
/// Safe on a corrupt or missing target. Drives the confirmation dialog's diff +
/// hard-stops. Deviation from the spec: no `state` argument — inspect touches no
/// session.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn inspect_backup(
    vault_path: String,
    archive_path: String,
) -> Result<BackupPreviewDto, CommandError> {
    let preview = inspect_backup_core(InspectBackupInput {
        target_vault: PathBuf::from(vault_path),
        archive_path: PathBuf::from(archive_path),
    })
    .await?;
    Ok(backup_preview_to_dto(preview))
}

/// Restore a vault from a `.vbk` (journaled, crash-safe).
///
/// Refuses an *unlocked* target — restore overwrites the `.vdb` out from under any
/// live session, and Windows holds the file open while a connection is live.
///
/// `confirm_rollback` is the user's explicit yes to the preview's rollback warning
/// (5.2c): when the live target is AHEAD of the backup, the core refuses unless this
/// is `true`. On success the keychain rollback baseline is re-based to the restored
/// counter so the next unlock does not nag.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn restore_vault(
    vault_path: String,
    archive_path: String,
    confirm_rollback: bool,
    state: tauri::State<'_, AppState>,
) -> Result<RestoreReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(
            "lock the vault before restoring".into(),
        ));
    }

    let factory = SqliteVaultRepositoryFactory::new();
    let report = restore_vault_core(
        &factory,
        state.keychain.as_ref(),
        RestoreVaultInput {
            target_vault: PathBuf::from(vault_path),
            archive_path: PathBuf::from(archive_path),
            confirm_rollback,
        },
    )
    .await?;
    Ok(restore_report_to_dto(report))
}

/// Convert a legacy `.vdb` + sibling-blobs vault to a `<name>.vedge/` home (slice 5.2.0).
///
/// A **file** operation — refuses an unlocked legacy vault (the migration copies then
/// reaps its files; Windows holds the `.vdb` open while a session is live). Crash-safe:
/// the original layout survives until the home is atomically committed. Returns the new
/// home path so the shell can re-point the recents row.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn convert_vault(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(
            "lock the vault before converting it".into(),
        ));
    }

    let out = migrate_vault_layout_core(
        state.keychain.as_ref(),
        MigrateVaultLayoutInput {
            legacy_vdb_path: PathBuf::from(vault_path),
        },
    )
    .await?;
    Ok(out.home.to_string_lossy().into_owned())
}
