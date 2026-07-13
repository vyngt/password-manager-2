//! Journaled, crash-safe restore of a vault from a `.vbk` archive (slice 5.2b).
//!
//! A **file** operation — no session, no KEK, no master password (you must be able
//! to restore a vault you cannot open). It verifies every checksum **before**
//! touching a live file, then runs the write-ahead swap in
//! [`crate::infrastructure::backup::journal`]. The Tauri command enforces the
//! "target must be locked" precondition; this use case enforces the identity,
//! format and schema refusals. On success it re-opens the restored vault through
//! the factory (which is a no-op recovery) and writes exactly one `BackupRestored`
//! audit row (audit rows are plaintext, so no key is needed).

use std::path::PathBuf;

use tracing::{instrument, warn};

use crate::application::vault::ports::factories::VaultRepositoryFactory;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::shared::{StorageError, format_rfc3339_millis, now};
use crate::domain::vault::entities::{AuditAction, AuditEvent, CURRENT_SCHEMA_VERSION};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::manifest::{BACKUP_FORMAT_VERSION, BLOBS_PREFIX, VAULT_MEMBER};
use crate::infrastructure::backup::target::read_target_identity;
use crate::infrastructure::backup::{archive, journal};

/// Which archive to restore, and over which vault.
#[derive(Debug, Clone)]
pub struct RestoreVaultInput {
    pub target_vault: PathBuf,
    pub archive_path: PathBuf,
    /// The user's explicit yes to the preview's rollback warning (slice 5.2c). When
    /// the live target is AHEAD of the backup, restore refuses unless this is `true`.
    pub confirm_rollback: bool,
}

/// Non-invertible derivatives surfaced to the UI — counts + identity, never any
/// key material or plaintext.
#[derive(Debug, Clone)]
pub struct RestoreReport {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub restored_at: String,
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Re-baseline the keychain rollback mirror to the RESTORED vault's own counter,
/// keyed on its uuid (ground truth over the manifest), so the very next unlock does
/// not nag about the rollback the user just chose. Best-effort: the swap already
/// committed durably, so a keychain failure (or a pre-4.6 backup with no uuid) must
/// NOT fail the restore (slice 5.2c).
async fn rebaseline_rollback_mirror(repo: &dyn VaultRepository, keychain: &dyn KeychainProvider) {
    let cfg = match repo.load_config().await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(error = %e, "restore committed but re-reading config for re-baseline failed");
            return;
        }
    };
    let Some(uuid) = cfg.vault_uuid.as_deref() else {
        return;
    };
    if let Err(e) = keychain.store_commit_baseline(uuid, cfg.commit_counter) {
        warn!(error = %e, "restore committed but the rollback baseline could not be re-based");
    }
}

#[instrument(skip_all, fields(target = %input.target_vault.display()))]
pub async fn restore_vault(
    repo_factory: &dyn VaultRepositoryFactory,
    keychain: &dyn KeychainProvider,
    input: RestoreVaultInput,
) -> Result<RestoreReport, VaultError> {
    let target = input.target_vault;
    let archive_path = input.archive_path;
    let confirm_rollback = input.confirm_rollback;

    // ---- 0. Reconcile any PRIOR interrupted restore of this vault -----------
    // A leftover journal + `.old` artifacts from an earlier crashed restore must be
    // resolved before a fresh swap, or the two collide (a second `move → .old` sees
    // both files and errors, wedging every future open). A no-op when nothing pends;
    // an `Err` here (a prior restore is unrecoverable) correctly aborts this one.
    journal::recover_if_pending(&target)?;

    // ---- PRE: verify + refuse (nothing live is touched) --------------------
    // 1. Integrity gate over the whole archive — names the offending member.
    let manifest = archive::verify_archive(&archive_path)?;

    // 2. Format / schema refusals (compared against constants, never a literal).
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(VaultError::MalformedPayload(format!(
            "unknown backup format version {} (this build writes {BACKUP_FORMAT_VERSION})",
            manifest.format_version
        )));
    }
    if manifest.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(VaultError::UnsupportedSchemaVersion(
            manifest.schema_version,
        ));
    }

    // 3. Wrong-vault refusal: compare the backup's uuid against the TARGET's
    //    plaintext `vault_config.vault_uuid` (read-only, no KEK). An unreadable
    //    target → identity unknown → allowed (the command already required a lock).
    if let Some(target_id) = read_target_identity(&target).await {
        if let (Some(backup_uuid), Some(target_uuid)) = (
            manifest.vault_uuid.as_deref(),
            target_id.vault_uuid.as_deref(),
        ) {
            if backup_uuid != target_uuid {
                return Err(VaultError::MalformedPayload(format!(
                    "this backup belongs to a different vault ({backup_uuid} ≠ {target_uuid})"
                )));
            }
        }
        // 4. Rollback gate (5.2c): a backup OLDER than the live target drops the
        //    intervening commits. Refuse unless the user confirmed the preview. An
        //    unknown target counter (pre-5.2c column) → can't compute → allow.
        if let Some(target_ctr) = target_id.commit_counter {
            if target_ctr > manifest.commit_counter && !confirm_rollback {
                return Err(VaultError::MalformedPayload(format!(
                    "this backup is older than the target vault (backup state {}, vault state {target_ctr}); confirm the rollback to proceed",
                    manifest.commit_counter
                )));
            }
        }
    }

    // (No cross-volume refusal: staging is `<target_parent>/.<stem>.restore-staging`
    //  by construction, so the staged→live rename is always intra-volume.)

    // ---- STAGE: extract + fsync + re-hash (still nothing live is touched) ---
    let staging_dir = journal::staging_path(&target);
    if staging_dir.exists() {
        // A stale staging dir from an aborted attempt with no journal — reap it.
        std::fs::remove_dir_all(&staging_dir).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(&staging_dir).map_err(|e| io_ctx("create staging", &e))?;

    let staged_vault = journal::staged_vault(&staging_dir);
    archive::extract_member(&archive_path, VAULT_MEMBER, &staged_vault)?;
    journal::fsync_file(&staged_vault)?;

    let staged_blobs = journal::staged_blobs(&staging_dir);
    for file in &manifest.files {
        let Some(rel) = file.name.strip_prefix(BLOBS_PREFIX) else {
            continue; // the vault member; handled above
        };
        // Blob members are flat `{ulid}.blob` names. Reject any path traversal in a
        // crafted archive — restore accepts an arbitrary user-chosen `.vbk`, so a
        // manifest member like `blobs/../../evil` must not escape the staging dir.
        if !journal::is_safe_blob_name(rel) {
            std::fs::remove_dir_all(&staging_dir).ok();
            return Err(io_msg(format!(
                "backup contains an unsafe blob member name: {}",
                file.name
            )));
        }
        std::fs::create_dir_all(&staged_blobs).map_err(|e| io_ctx("create staged blobs", &e))?;
        let dest = staged_blobs.join(rel);
        archive::extract_member(&archive_path, &file.name, &dest)?;
        journal::fsync_file(&dest)?;
    }
    journal::fsync_dir(&staging_dir);

    // Re-hash the extracted snapshot against the manifest — this value anchors the
    // journal and is what recovery re-verifies before trusting the snapshot.
    let expected_hash = manifest
        .files
        .iter()
        .find(|m| m.name == VAULT_MEMBER)
        .map(|m| m.blake3.clone())
        .ok_or_else(|| io_msg("manifest is missing the vault member"))?;
    let (_size, vault_hash) = archive::hash_file(&staged_vault)?;
    if vault_hash != expected_hash {
        std::fs::remove_dir_all(&staging_dir).ok();
        return Err(io_msg("staged snapshot hash mismatch after extraction"));
    }

    // ---- COMMIT: the write-ahead swap (live files change here) --------------
    let started_at = format_rfc3339_millis(now());
    journal::commit(&target, &staging_dir, &vault_hash, &started_at)?;

    // ---- POST: re-open (recovery is now a no-op) + one audit row -----------
    // The swap has committed durably; the audit row is bookkeeping. A reopen failure
    // is fatal (it signals a bad restore), but a failed audit write must NOT report a
    // successful restore as failed — log it and carry on.
    let repo = repo_factory.open(&target).await?;
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::BackupRestored,
        occurred_at: now(),
        device_id: None,
    };
    if let Err(e) = repo.append_audit(&event).await {
        warn!(error = %e, "restore committed but the audit row could not be written");
    }

    rebaseline_rollback_mirror(repo.as_ref(), keychain).await;

    Ok(RestoreReport {
        vault_uuid: manifest.vault_uuid.clone(),
        entry_count: manifest.entry_count,
        blob_count: manifest.blob_count,
        restored_at: started_at,
    })
}
