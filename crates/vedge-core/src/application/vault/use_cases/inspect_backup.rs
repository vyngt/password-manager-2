//! Preview a `.vbk` before restoring it (slice 5.2b) — no session, no KEK, no writes.
//!
//! Reads the archive's manifest and the **target** vault's plaintext identity
//! (read-only), and computes the same hard-stops [`super::restore_vault`] enforces
//! so the UI can render a *blocked* dialog instead of discovering a refusal only at
//! restore time. Safe to call on a missing or corrupt target. The commit-counter /
//! rollback fields are wired in slice 5.2c (they need the `commit_counter` column).

use std::path::PathBuf;

use tracing::instrument;

use crate::domain::vault::entities::CURRENT_SCHEMA_VERSION;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive;
use crate::infrastructure::backup::manifest::BACKUP_FORMAT_VERSION;
use crate::infrastructure::backup::target::read_target_identity;

/// Which archive to preview, and against which target vault.
#[derive(Debug, Clone)]
pub struct InspectBackupInput {
    pub target_vault: PathBuf,
    pub archive_path: PathBuf,
}

/// What restoring this backup would do — backup facts, target facts, and the
/// computed hard-stops. Derivatives only; no key material.
// The four bools are distinct, independent hard-stops (each maps to its own refusal
// reason), not a state that a single enum could express.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct BackupPreview {
    // ---- the backup ----
    pub format_version: u32,
    pub backup_vault_uuid: Option<String>,
    pub backup_schema_version: i32,
    pub created_at: String,
    pub backup_entry_count: u64,
    pub backup_blob_count: u64,
    // ---- the current target ----
    pub target_uuid: Option<String>,
    pub target_entry_count: Option<u64>,
    pub target_last_unlocked_at: Option<String>,
    /// `None` in 5.2b (no `commit_counter` column yet); populated in 5.2c.
    pub target_commit_counter: Option<i64>,
    // ---- hard stops (any true = restore refuses) ----
    pub uuid_mismatch: bool,
    pub unknown_format: bool,
    pub unknown_schema: bool,
    /// Target missing/corrupt → identity unknown → allowed behind a second confirm.
    pub target_unreadable: bool,
    /// The rollback framing (5.2c): restoring to N while this device knew M.
    pub rollback_delta: Option<i64>,
}

#[instrument(skip_all, fields(target = %input.target_vault.display()))]
pub async fn inspect_backup(input: InspectBackupInput) -> Result<BackupPreview, VaultError> {
    let manifest = archive::read_manifest(&input.archive_path)?;

    let unknown_format = manifest.format_version != BACKUP_FORMAT_VERSION;
    let unknown_schema = manifest.schema_version > CURRENT_SCHEMA_VERSION;

    let target = read_target_identity(&input.target_vault).await;
    let target_unreadable = target.is_none();
    let (target_uuid, target_entry_count, target_last_unlocked_at, target_commit_counter) = target
        .map_or((None, None, None, None), |t| {
            (
                t.vault_uuid,
                Some(t.entry_count),
                t.last_unlocked_at,
                t.commit_counter,
            )
        });

    let uuid_mismatch = match (manifest.vault_uuid.as_deref(), target_uuid.as_deref()) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };

    // Rollback framing (5.2c): restoring this backup takes the vault back to the
    // backup's `commit_counter`. If the live target is AHEAD, the delta is how much
    // state the restore would drop. Only computable when the target's counter is
    // readable AND strictly greater than the backup's.
    let rollback_delta = match target_commit_counter {
        Some(t) if t > manifest.commit_counter => Some(t.saturating_sub(manifest.commit_counter)),
        _ => None,
    };

    Ok(BackupPreview {
        format_version: manifest.format_version,
        backup_vault_uuid: manifest.vault_uuid,
        backup_schema_version: manifest.schema_version,
        created_at: manifest.created_at,
        backup_entry_count: manifest.entry_count,
        backup_blob_count: manifest.blob_count,
        target_uuid,
        target_entry_count,
        target_last_unlocked_at,
        target_commit_counter,
        uuid_mismatch,
        unknown_format,
        unknown_schema,
        target_unreadable,
        rollback_delta,
    })
}
