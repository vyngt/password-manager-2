//! Backup commands (slices 5.2 / 5.2.2) — Decision ⑦'s three verbs, plus the conversion.
//!
//! | command | session? | destructive? |
//! |---|---|---|
//! | `backup_vault` | **unlocked** (the snapshot runs through its repo) | no |
//! | `inspect_backup` | none — a read-only preview | no |
//! | `open_backup` | none — a file operation | 🟢 **never**: it refuses an occupied path |
//! | `replace_vault_from_backup` | none, but the target must be **locked** | 🔴 yes |
//! | `backup_status` | **unlocked** | no |
//!
//! `backup_vault` holds its guard across the archive write (see `commands/maintenance.rs` for
//! the `#![allow]` rationale).
//!
//! 🔴 **`open_backup` and `replace_vault_from_backup` must never be collapsed back into one
//! command.** They were one command in 5.2, and it took four confirmation dialogs to make it
//! survivable — three of which existed *only* because an import could be aimed at an existing
//! file. Two verbs, two names, two buttons: that is the fix, and it is structural.

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
    BackupVaultInput, InspectBackupInput, MigrateVaultLayoutInput, OpenBackupInput,
    ReplaceVaultInput, backup_status as backup_status_core, backup_vault as backup_vault_core,
    inspect_backup as inspect_backup_core, list_recent_vaults,
    migrate_vault_layout as migrate_vault_layout_core, open_backup as open_backup_core,
    replace_vault_from_backup as replace_vault_from_backup_core,
};

use crate::dto::backup::{
    BackupPreviewDto, BackupReportDto, BackupStatusDto, ConvertVaultResultDto, OpenBackupReportDto,
    ReplaceReportDto, backup_preview_to_dto, backup_report_to_dto, open_backup_report_to_dto,
    replace_report_to_dto,
};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// Every vault home this machine knows about, for ②'s duplicate check.
///
/// The **shell** supplies these: `recent_vaults` lives in `app.db`, and a *vault* use case must
/// not reach into the app DB to find out what else is installed. `open_backup` and
/// `inspect_backup` take the list as an argument and probe it themselves.
async fn known_vaults(state: &AppState) -> Vec<PathBuf> {
    match list_recent_vaults(&*state.recent_vaults).await {
        Ok(rows) => rows.into_iter().map(|r| r.path).collect(),
        Err(e) => {
            // A duplicate we fail to notice mints no fresh uuid, which would let a copy share
            // its original's rollback baseline. Loud, and degraded — never silent.
            tracing::warn!(error = %e, "could not read recents; ② duplicate detection is degraded");
            Vec::new()
        }
    }
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

/// Preview a `.vbk` — read-only, no session, safe on a corrupt or missing target.
///
/// One preview, two callers (Decision ⑦):
/// - **Open backup** passes `dest_home` (is the path free? is this a duplicate?)
/// - **Replace** passes `target_vault` (does it match? would it roll back?)
///
/// Both get M3's credential answer. Every hard-stop it reports is one the use case re-checks
/// for itself — a preview is never a gate.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(archive = %archive_path))]
pub async fn inspect_backup(
    archive_path: String,
    target_vault: Option<String>,
    dest_home: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<BackupPreviewDto, CommandError> {
    let preview = inspect_backup_core(InspectBackupInput {
        archive_path: PathBuf::from(archive_path),
        target_vault: target_vault.map(PathBuf::from),
        dest_home: dest_home.map(PathBuf::from),
        known_vaults: known_vaults(&state).await,
    })
    .await?;
    Ok(backup_preview_to_dto(preview))
}

/// 🟢 **Open a backup at a path that has nothing there.** The everyday verb, and the one that
/// cannot destroy anything: an occupied destination is refused outright, with no override.
///
/// No session and no lock check, because there is nothing here to lock — the destination does
/// not exist yet, and if it does, this refuses.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(dest = %dest_home))]
pub async fn open_backup(
    archive_path: String,
    dest_home: String,
    state: tauri::State<'_, AppState>,
) -> Result<OpenBackupReportDto, CommandError> {
    let report = open_backup_core(
        state.keychain.as_ref(),
        OpenBackupInput {
            archive_path: PathBuf::from(archive_path),
            dest_home: PathBuf::from(dest_home),
            known_vaults: known_vaults(&state).await,
        },
    )
    .await?;
    Ok(open_backup_report_to_dto(report))
}

/// 🔴 **Replace a live vault's contents with a `.vbk`** — the demoted escape hatch.
///
/// Refuses an *unlocked* target: the swap renames the home out from under any live session, and
/// Windows holds the `.vdb` open while a connection is live.
///
/// The three `confirm_*` flags are three **independent** acknowledgements of three independent
/// risks (rollback · credentials · an unidentifiable target). The GUI must never collapse them
/// into one checkbox: a user who has understood that this rolls their vault back has not thereby
/// understood that it needs a password they may no longer possess.
///
/// A uuid mismatch has no flag. It is a hard refusal.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn replace_vault_from_backup(
    vault_path: String,
    archive_path: String,
    confirm_rollback: bool,
    confirm_credential_change: bool,
    confirm_unverified_target: bool,
    state: tauri::State<'_, AppState>,
) -> Result<ReplaceReportDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(
            "lock the vault before replacing it".into(),
        ));
    }

    let factory = SqliteVaultRepositoryFactory::new();
    let report = replace_vault_from_backup_core(
        &factory,
        state.keychain.as_ref(),
        ReplaceVaultInput {
            target_vault: PathBuf::from(vault_path),
            archive_path: PathBuf::from(archive_path),
            confirm_rollback,
            confirm_credential_change,
            confirm_unverified_target,
        },
    )
    .await?;
    Ok(replace_report_to_dto(report))
}

/// The vault's backup health (Decision ⑧).
///
/// 🔴 `last_backup_at` is written **only** by `backup_vault`. It is deliberately NOT derived
/// from `BackupCreated` audit rows, because `create_snapshot` emits those too (5.2.1) — and a
/// snapshot must never be allowed to silence *"you have never backed up."* A snapshot lives on
/// the same disk as the vault and dies with it; that is the whole reason both verbs exist.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn backup_status(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<BackupStatusDto, CommandError> {
    let handle = state.get_session(&vault_id_from_string(&vault_path))?;
    let guard = handle.lock().await;
    let status = backup_status_core(&guard).await?;
    Ok(BackupStatusDto {
        last_backup_at: status.last_backup_at.map(|t| t.to_string()),
        last_snapshot_at: status.last_snapshot_at.map(|t| t.to_string()),
    })
}

/// Convert a legacy `.vdb` + sibling-blobs vault to a `<name>.vedge/` home (slice 5.2.0).
///
/// A **file** operation — refuses an unlocked legacy vault (the migration copies then
/// reaps its files; Windows holds the `.vdb` open while a session is live). Crash-safe:
/// the original layout survives until the home is atomically committed. Returns the new
/// home path (so the shell can re-point the recents row) and whether a legacy biometric
/// credential was purged (slice 5.2.3 — the UI then prompts to re-enable Hello).
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn convert_vault(
    vault_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<ConvertVaultResultDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    if state.is_unlocked(&vault_id) {
        return Err(CommandError::Invalid(
            "lock the vault before converting it".into(),
        ));
    }

    let out = migrate_vault_layout_core(
        state.keychain.as_ref(),
        state.biometric.as_ref(),
        MigrateVaultLayoutInput {
            legacy_vdb_path: PathBuf::from(vault_path),
        },
    )
    .await?;
    Ok(ConvertVaultResultDto {
        home: out.home.to_string_lossy().into_owned(),
        biometric_reset: out.biometric_reset,
    })
}
