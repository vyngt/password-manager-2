//! Backup / restore commands (slice 5.2). Mirrors
//! `vedge-tauri/src/commands/backup.rs`.
//!
//! `backup` needs an unlocked session (the live snapshot runs through it); `inspect`
//! is a read-only preview; `restore` is a **file** op that refuses an unlocked target
//! (the caller locks first) and re-bases the rollback baseline on success.

use serde::Serialize;

use vedge_ipc::{BackupPreviewDto, BackupReportDto, RestoreReportDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Back up the (unlocked) vault to a `.vbk` archive at `dest_path`.
pub async fn backup(vault_path: &str, dest_path: &str) -> Result<BackupReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        dest_path: &'a str,
    }
    call(
        "backup_vault",
        &Args {
            vault_path,
            dest_path,
        },
    )
    .await
}

/// Preview what restoring `archive_path` over `vault_path` would do (read-only; safe
/// on a corrupt or missing target).
pub async fn inspect(vault_path: &str, archive_path: &str) -> Result<BackupPreviewDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        archive_path: &'a str,
    }
    call(
        "inspect_backup",
        &Args {
            vault_path,
            archive_path,
        },
    )
    .await
}

/// Restore `vault_path` from `archive_path`. The target must be **locked**;
/// `confirm_rollback` is the user's explicit yes to a rollback (restoring an older
/// backup over a newer vault).
pub async fn restore(
    vault_path: &str,
    archive_path: &str,
    confirm_rollback: bool,
) -> Result<RestoreReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        archive_path: &'a str,
        confirm_rollback: bool,
    }
    call(
        "restore_vault",
        &Args {
            vault_path,
            archive_path,
            confirm_rollback,
        },
    )
    .await
}
