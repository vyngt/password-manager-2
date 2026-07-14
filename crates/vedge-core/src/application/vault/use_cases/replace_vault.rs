//! Replace a vault's contents with a `.vbk` archive — **the demoted escape hatch**
//! (slice 5.2.2; was `restore_vault` in 5.2b).
//!
//! ## Why this is not the default
//!
//! Decision ⑦ split 5.2's single "restore" verb in two. The everyday verb is
//! [`super::open_backup`], which materialises a backup at a path that has **nothing there**
//! and therefore cannot destroy anything. This one overwrites a live vault, and exists only
//! because a 200 GB vault cannot afford the transient second copy that *Open backup* needs.
//! It lives behind an **Advanced ▸** disclosure and is never the first thing a frightened
//! user sees.
//!
//! Three of 5.2's four confirmation dialogs existed **only** because an import was pointed at
//! an existing file. They do not disappear — they **move here**, where they belong to exactly
//! one operation:
//!
//! | Guard | Behaviour |
//! |---|---|
//! | format (**H1**) | refuse **newer**; an older `.vbk` always restores |
//! | schema | refuse newer |
//! | target missing | not an error — that is *Open backup*; redirect |
//! | target **unreadable** (**H2**) | its **own** acknowledgement; never a silent skip |
//! | uuid mismatch | 🔴 **hard refusal.** No confirm. The wrong vault is the wrong vault. |
//! | rollback (`commit_counter`) | its own acknowledgement |
//! | credentials differ (**M3**) | its own acknowledgement |
//!
//! **Three independent risks ⇒ three independent acknowledgements.** Never one checkbox for
//! two of them: a user who has understood the rollback has not thereby understood that the
//! backup needs a password they may no longer have.
//!
//! ## And it is undoable
//!
//! 🟢 It auto-snapshots the live vault first (Decision ⑭) and commits through
//! `journal::commit_preserving(&[SNAPSHOTS_DIR])`, so the `snapshots/` store — which lives
//! INSIDE the home being swapped — survives, undo point and all. A plain `journal::commit`
//! would delete the very snapshot that makes this reversible.
//!
//! A **file** operation: no session, no KEK, no master password (you must be able to replace
//! a vault you cannot open). The Tauri command enforces "the target must be locked".

use std::path::PathBuf;

use tracing::{instrument, warn};

use crate::application::vault::ports::factories::VaultRepositoryFactory;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::domain::shared::{SNAPSHOTS_DIR, format_rfc3339_millis, now};
use crate::domain::vault::entities::{AuditAction, AuditEvent, CURRENT_SCHEMA_VERSION};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::manifest::{BACKUP_FORMAT_VERSION, BackupManifest};
use crate::infrastructure::backup::target::{TargetState, read_target_state};
use crate::infrastructure::backup::{archive, journal};

use super::open_backup::stage_home_from_archive;
use super::revert_to_snapshot::{auto_snapshot_pre_restore, commit_swap_blocking};

/// Which archive to write over which vault — and the user's three separate yeses.
#[derive(Debug, Clone)]
pub struct ReplaceVaultInput {
    /// The vault HOME directory (`<name>.vedge/`) to overwrite. Must exist.
    pub target_vault: PathBuf,
    pub archive_path: PathBuf,
    /// Yes to: this backup is OLDER than the live vault, and replacing drops the difference.
    pub confirm_rollback: bool,
    /// Yes to: this backup was sealed with a different master password / Secret Key (**M3**),
    /// so the replaced vault will not open with the credentials in use today.
    pub confirm_credential_change: bool,
    /// Yes to: the target could not be read, so `VEdge` could **not** verify this backup even
    /// belongs to it (**H2**). The uuid guard is *inoperable*, not *satisfied*.
    pub confirm_unverified_target: bool,
}

/// Non-invertible derivatives surfaced to the UI — counts + identity, never key material.
#[derive(Debug, Clone)]
pub struct ReplaceReport {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub restored_at: String,
    /// ⑭: the `pre-restore` snapshot holding the state this replace overwrote — the undo
    /// point. `None` only when the target was unreadable (there was nothing to preserve).
    pub undo_snapshot_id: Option<String>,
}

/// Re-baseline the keychain rollback mirror to the REPLACED vault's own counter, keyed on its
/// uuid (ground truth over the manifest), so the very next unlock does not nag about the
/// rollback the user just chose. Best-effort: the swap already committed durably, so a
/// keychain failure (or a pre-4.6 backup with no uuid) must NOT fail the operation (L1).
async fn rebaseline_rollback_mirror(repo: &dyn VaultRepository, keychain: &dyn KeychainProvider) {
    let cfg = match repo.load_config().await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(error = %e, "replace committed but re-reading config for re-baseline failed");
            return;
        }
    };
    let Some(uuid) = cfg.vault_uuid.as_deref() else {
        return;
    };
    if let Err(e) = keychain.store_commit_baseline(uuid, cfg.commit_counter) {
        warn!(error = %e, "replace committed but the rollback baseline could not be re-based");
    }
}

/// Every refusal that depends on what is actually AT the target.
///
/// Returns the target's ACTIVE-entry count when it was readable — the caller needs it for the ⑭
/// auto-snapshot — or `None` when the target is unreadable (and the user has said so is fine).
///
/// 🔴 The whole of finding H2 lives in this function's SHAPE. In 5.2b these checks sat inside
/// an `if let Some(identity)`, so an unreadable target fell straight through **all** of them
/// — the uuid guard disarmed itself on precisely the vaults most likely to be replaced.
/// Matching on [`TargetState`] makes that impossible: `Unreadable` is a branch you must
/// answer for.
async fn check_target(
    home: &std::path::Path,
    manifest: &BackupManifest,
    input: &ReplaceVaultInput,
) -> Result<Option<u64>, VaultError> {
    let identity = match read_target_state(home).await {
        // Nothing here to replace. Not a scary edge case needing a confirmation — a different
        // verb. Say so, and let the UI send them to "Open a backup".
        TargetState::Missing => return Err(VaultError::TargetMissing),

        // There IS a vault here, but we could not identify it, so we CANNOT know whether this
        // backup belongs to it. Its own risk, its own acknowledgement — never a silent pass.
        TargetState::Unreadable => {
            if !input.confirm_unverified_target {
                return Err(VaultError::TargetUnverified);
            }
            // Nothing readable ⇒ no rollback compare, no credential compare, and no ⑭
            // auto-snapshot (a snapshot of an unreadable vault is not an undo point).
            warn!("replacing an UNIDENTIFIED target — the uuid guard could not run (confirmed)");
            return Ok(None);
        }

        TargetState::Readable(id) => id,
    };

    // 🔴 Wrong vault. A HARD refusal — there is no confirmation for this, because there is no
    // legitimate reason to write vault B's contents over vault A. A user who genuinely wants
    // B's data on this machine wants *Open backup*, at a path of its own.
    if let (Some(backup_uuid), Some(target_uuid)) = (
        manifest.vault_uuid.as_deref(),
        identity.vault_uuid.as_deref(),
    ) && backup_uuid != target_uuid
    {
        return Err(VaultError::BackupWrongVault {
            backup: backup_uuid.to_owned(),
            target: target_uuid.to_owned(),
        });
    }

    // Risk 1 — rollback: the live vault is AHEAD of the backup, so replacing drops the
    // difference. An unknown target counter (pre-5.2c column) → can't compute → allow.
    if let Some(target_ctr) = identity.commit_counter
        && target_ctr > manifest.commit_counter
        && !input.confirm_rollback
    {
        return Err(VaultError::RollbackNotConfirmed);
    }

    // Risk 2 — credentials (M3): the backup was sealed under a different master password /
    // Secret Key, so after the swap the vault will not open with today's credentials.
    // 🔴 Fires only when BOTH prefixes are known. An unknown prefix means UNKNOWN, never
    // "same" — a pre-5.2.2 archive carries none, and asserting a match we never verified is
    // how a user ends up locked out of their own vault holding a kit they were told they no
    // longer needed.
    if let (Some(backup_vh), Some(target_vh)) = (
        manifest.verify_hash_prefix.as_deref(),
        identity.verify_hash_prefix.as_deref(),
    ) && backup_vh != target_vh
        && !input.confirm_credential_change
    {
        return Err(VaultError::BackupCredentialsDiffer);
    }

    // Every refusal has passed. Hand the caller what it needs for the ⑭ snapshot, which it takes
    // only once staging has succeeded (see the call site).
    Ok(Some(identity.entry_count))
}

#[instrument(skip_all, fields(target = %input.target_vault.display()))]
pub async fn replace_vault_from_backup(
    repo_factory: &dyn VaultRepositoryFactory,
    keychain: &dyn KeychainProvider,
    input: ReplaceVaultInput,
) -> Result<ReplaceReport, VaultError> {
    let target = input.target_vault.clone();
    let archive_path = input.archive_path.clone();

    // ---- 0. Reconcile any PRIOR interrupted swap of this vault -------------------
    // Leftover journal + `.old` artifacts from an earlier crashed operation must be resolved
    // before a fresh swap, or the two collide (a second `move → .old` sees both and errors,
    // wedging every future open). A no-op when nothing pends; an `Err` here (a prior restore
    // is unrecoverable) correctly aborts this one.
    journal::recover_if_pending(&target)?;

    // ---- PRE: verify + refuse (nothing live is touched) --------------------------
    // 1. Integrity gate over the whole archive — per-member hashes, plus the `.blake3`
    //    sidecar when present (L2, slice 5.2.2). Names the offending member.
    let manifest = archive::verify_archive(&archive_path)?;

    // 2. Format / schema refusals (compared against constants, never a literal).
    //    🔴 H1: refuse NEWER, accept OLDER. An older `.vbk` must restore forever, or the day
    //    the format bumps, every backup on every disk is bricked.
    if manifest.format_version > BACKUP_FORMAT_VERSION {
        return Err(VaultError::BackupUnsupportedFormat(manifest.format_version));
    }
    if manifest.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(VaultError::UnsupportedSchemaVersion(
            manifest.schema_version,
        ));
    }

    // 3. Everything that depends on the target: identity, rollback, credentials.
    let target_entry_count = check_target(&target, &manifest, &input).await?;

    // ---- STAGE: extract + fsync + re-hash (still nothing live is touched) ---------
    // (No cross-volume refusal: staging is `<parent>/.<stem>.restore-staging` by
    //  construction, so the staged→live rename is always intra-volume.)
    let staging_dir = journal::staging_path(&target);
    let staged = stage_home_from_archive(&archive_path, &manifest, &staging_dir)?;

    // 🟢 ⑭ The undo point — taken HERE, after staging has succeeded and immediately before the
    // swap, not back with the guards.
    //
    // Ordering matters: staging can still fail (an unsafe blob member, a full disk, a truncated
    // archive), and every one of those paths returns without touching the live vault. A snapshot
    // taken before staging would therefore leave a `pre-restore` undo point in the store for an
    // operation that never happened — offering the user, on the Snapshots page, an "undo" of
    // nothing. The store should only ever record what actually occurred.
    //
    // `None` ⇒ the target was unreadable ⇒ there is nothing to preserve, and a snapshot of an
    // unreadable vault would not be an undo point.
    let undo_snapshot_id = match target_entry_count {
        Some(entry_count) => {
            auto_snapshot_pre_restore(&target, &target.join(SNAPSHOTS_DIR), entry_count).await
        }
        None => None,
    };

    // ---- COMMIT: the write-ahead swap (live files change here) --------------------
    // 🔴 `commit_swap_blocking` is `commit_preserving(&[SNAPSHOTS_DIR])`. The snapshot store
    // lives INSIDE the home, so a plain whole-home swap would delete it — including the undo
    // point taken seconds ago, which would make ⑭ a lie. Do NOT "simplify" this to
    // `journal::commit`; `replace_preserves_the_snapshot_store` fails if you do.
    let started_at = format_rfc3339_millis(now());
    commit_swap_blocking(&target, &staging_dir, &staged.vault_blake3, &started_at).await?;

    // ---- POST (L1): the swap committed durably. NOTHING below may fail it ---------
    // The reopen IS fatal (it signals a bad restore); the audit row and the re-baseline are
    // bookkeeping, and a failure there must not report a successful replace as failed.
    let repo = repo_factory.open(&target).await?;
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::BackupRestored,
        occurred_at: now(),
        device_id: None,
    };
    if let Err(e) = repo.append_audit(&event).await {
        warn!(error = %e, "replace committed but the audit row could not be written");
    }

    rebaseline_rollback_mirror(repo.as_ref(), keychain).await;

    Ok(ReplaceReport {
        vault_uuid: manifest.vault_uuid.clone(),
        entry_count: manifest.entry_count,
        blob_count: manifest.blob_count,
        restored_at: started_at,
        undo_snapshot_id,
    })
}
