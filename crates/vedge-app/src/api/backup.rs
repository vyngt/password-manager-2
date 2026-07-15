//! Backup commands (slices 5.2 / 5.2.2). Mirrors `vedge-tauri/src/commands/backup.rs`.
//!
//! Decision ⑦'s three verbs, and only one of them can destroy anything:
//!
//! - [`backup`] — write a `.vbk` anywhere (needs an unlocked session).
//! - [`open`] — materialise a vault at a path with **nothing there**. 🟢 Cannot overwrite.
//! - [`replace`] — the demoted escape hatch. 🔴 Overwrites a live vault; needs it **locked**,
//!   and takes three *independent* acknowledgements.
//!
//! [`inspect`] previews for both, and [`status`] answers "when did I last actually back up?"

use serde::Serialize;

use vedge_ipc::{
    BackupPreviewDto, BackupReportDto, BackupStatusDto, OpenBackupReportDto, ReplaceReportDto,
};

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

/// Preview an archive. Read-only; safe on a corrupt or missing target.
///
/// Pass `dest_home` for the **Open** flow (is the path free? is this a duplicate? do the
/// credentials match?) or `target_vault` for the **Replace** flow (does it match this vault?
/// would it roll back?). One preview, two callers.
pub async fn inspect(
    archive_path: &str,
    target_vault: Option<&str>,
    dest_home: Option<&str>,
) -> Result<BackupPreviewDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        archive_path: &'a str,
        target_vault: Option<&'a str>,
        dest_home: Option<&'a str>,
    }
    call(
        "inspect_backup",
        &Args {
            archive_path,
            target_vault,
            dest_home,
        },
    )
    .await
}

/// 🟢 Open `archive_path` as a new vault at `dest_home`.
///
/// `dest_home` **must not exist** — the backend refuses otherwise, and there is no override.
/// That single constraint is what makes this the safe, everyday verb.
pub async fn open(archive_path: &str, dest_home: &str) -> Result<OpenBackupReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        archive_path: &'a str,
        dest_home: &'a str,
    }
    call(
        "open_backup",
        &Args {
            archive_path,
            dest_home,
        },
    )
    .await
}

/// 🔴 Replace `vault_path`'s contents with `archive_path`. **Destructive.** The target must be
/// **locked** (the caller locks first).
///
/// The three confirms are three *independent* risks — never bind them to one checkbox:
/// - `confirm_rollback`: the backup is older than the live vault; the difference is dropped.
/// - `confirm_credential_change`: the backup needs the master password / Secret Key in force
///   when it was taken — possibly ones the user no longer has.
/// - `confirm_unverified_target`: the target could not be read, so `VEdge` could not check that
///   this backup even belongs to it.
///
/// A uuid mismatch has no confirm. It is refused outright.
pub async fn replace(
    vault_path: &str,
    archive_path: &str,
    confirm_rollback: bool,
    confirm_credential_change: bool,
    confirm_unverified_target: bool,
) -> Result<ReplaceReportDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        archive_path: &'a str,
        confirm_rollback: bool,
        confirm_credential_change: bool,
        confirm_unverified_target: bool,
    }
    call(
        "replace_vault_from_backup",
        &Args {
            vault_path,
            archive_path,
            confirm_rollback,
            confirm_credential_change,
            confirm_unverified_target,
        },
    )
    .await
}

/// When this vault was last **backed up** (and, separately, last snapshotted).
///
/// 🔴 A snapshot is not a backup — it lives on the same disk and dies with it. The two are
/// distinct fields precisely so the UI cannot accidentally let one reassure the user about the
/// other (Decision ⑧).
pub async fn status(vault_path: &str) -> Result<BackupStatusDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
    }
    call("backup_status", &Args { vault_path }).await
}
