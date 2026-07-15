//! Revert a vault IN PLACE to one of its local snapshots (slice 5.2.1) — the destructive
//! half of Decision ⑦'s TIME axis, and undoable.
//!
//! A **file** operation, like `restore_vault`: no session, no KEK (you must be able to revert
//! a vault you cannot open — the H0 disaster path). It reuses 5.2b's journaled crash-safe
//! swap, but with two differences from a `.vbk` restore:
//!
//! 1. 🟢 It **auto-snapshots the current vault first** (Decision ⑭), tagged `pre-restore`, so
//!    a wrong revert is itself revertible. Skipped only when the target is unreadable — there
//!    is nothing to preserve.
//! 2. 🔴 It swaps with `journal::commit_preserving(&[SNAPSHOTS_DIR])` so the home's
//!    `snapshots/` store SURVIVES the swap (Decision ⑮). The store lives inside the home a
//!    revert replaces — a plain whole-home swap would delete the safety net it is made of.
//!
//! The snapshot is self-verifying (each object's BLAKE3 == its own filename), so a corrupt
//! target disarms no guard: the authority is the snapshot's manifest, and the store is ours.

use std::path::PathBuf;

use tracing::{instrument, warn};
use zeroize::Zeroizing;

use crate::application::vault::ports::factories::VaultRepositoryFactory;
use crate::application::vault::ports::keychain::KeychainProvider;
use crate::application::vault::ports::repository::VaultRepository;
use crate::application::vault::session::VaultSession;
use crate::domain::shared::{
    BLOBS_DIR, SNAPSHOTS_DIR, StorageError, VAULT_FILE, format_rfc3339_millis, now,
};
use crate::domain::vault::crypto_constants::KEK_LEN;
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::target::read_target_identity;
use crate::infrastructure::backup::{archive, journal};
use crate::infrastructure::snapshot::manifest::{
    SNAPSHOT_FORMAT_VERSION, SnapshotManifest, SnapshotReason,
};
use crate::infrastructure::snapshot::store;
use crate::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

use super::create_snapshot::write_snapshot;
use super::lock_vault::close_session_db;
use super::unlock_vault::UnlockVault;

/// Which snapshot to revert this vault to.
#[derive(Debug, Clone)]
pub struct RevertToSnapshotInput {
    /// The vault HOME directory (`<name>.vedge/`).
    pub vault: PathBuf,
    /// The snapshot's directory name (its id). UNTRUSTED — resolved + containment-checked.
    pub snapshot_id: String,
    /// The user's explicit yes to the rollback (reverting is a rollback by design). When the
    /// live vault is AHEAD of the snapshot, revert refuses unless this is `true` — ⑭ makes it
    /// undoable, so this is the expected case, not a hazard.
    pub confirm_rollback: bool,
}

/// Non-invertible facts surfaced to the UI.
#[derive(Debug, Clone)]
pub struct RevertReport {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis UTC (fixed-width).
    pub reverted_at: String,
}

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

/// Re-baseline the keychain rollback mirror to the REVERTED vault's own counter (best-effort;
/// the swap already committed, so a keychain failure must not fail the revert — L1). Mirrors
/// `restore_vault::rebaseline_rollback_mirror`.
async fn rebaseline_rollback_mirror(repo: &dyn VaultRepository, keychain: &dyn KeychainProvider) {
    let cfg = match repo.load_config().await {
        Ok(cfg) => cfg,
        Err(e) => {
            warn!(error = %e, "revert committed but re-reading config for re-baseline failed");
            return;
        }
    };
    let Some(uuid) = cfg.vault_uuid.as_deref() else {
        return;
    };
    if let Err(e) = keychain.store_commit_baseline(uuid, cfg.commit_counter) {
        warn!(error = %e, "revert committed but the rollback baseline could not be re-based");
    }
}

/// Decision ⑭: capture a `pre-restore` snapshot of the CURRENT vault before a destructive
/// swap, so the operation is undoable. Returns the new snapshot's id, or `None` if it was
/// skipped. Best-effort — a failure (a corrupt/unreadable target) is logged and skipped,
/// never fatal.
///
/// Shared by `revert_to_snapshot` (5.2.1) and `replace_vault_from_backup` (5.2.2): both
/// overwrite a live vault, and ⑭ says both must be undoable.
///
/// 🔴 Opens a `VaultDbConnection` DIRECTLY and `close()`s it before returning, rather than a
/// port `Arc<dyn VaultRepository>` — the connection MUST be released before the swap renames
/// the home, and Windows will not rename a directory holding an open `.vdb` handle. sqlx's
/// `Drop` close is async, so an explicit close is the only guarantee (M1's retry is the
/// backstop, not the mechanism).
pub(super) async fn auto_snapshot_pre_restore(
    home: &std::path::Path,
    snapshots_dir: &std::path::Path,
    entry_count: u64,
) -> Option<String> {
    let db = match VaultDbConnection::open(&home.join(VAULT_FILE)).await {
        Ok(db) => db,
        Err(e) => {
            warn!(error = %e, "target unreadable — skipping the pre-restore auto-snapshot (⑭)");
            return None;
        }
    };
    let repo = SqliteVaultRepository::new(db.handle());
    let blobs_dir = home.join(BLOBS_DIR);
    let id = match write_snapshot(
        snapshots_dir,
        &blobs_dir,
        &repo,
        SnapshotReason::PreRestore,
        entry_count,
    )
    .await
    {
        Ok(report) => Some(report.id),
        Err(e) => {
            warn!(error = %e, "pre-restore auto-snapshot failed — proceeding anyway (⑭)");
            None
        }
    };
    drop(repo); // release the repo's handle clone so `close()` can actually close the pool
    if let Err(e) = db.close().await {
        warn!(error = %e, "closing the pre-restore connection failed before the swap");
    }
    id
}

/// Run the snapshot-preserving swap on a blocking thread.
///
/// 🔴 `commit_preserving(&[SNAPSHOTS_DIR])`, never `journal::commit` — the `snapshots/` store
/// lives INSIDE the home being swapped, so a plain whole-home swap deletes it, including the
/// ⑭ undo point taken moments earlier. Both destructive verbs (revert, replace) go through
/// here precisely so neither can forget.
///
/// The swap is sync, and right after an in-place revert follows a lock its rename may block for
/// a while as the just-locked session's sqlx pool finishes releasing the `.vdb` handle on
/// Windows (M1). Running it on a blocking thread keeps the async runtime free to FINISH that
/// pool close while the rename retries — the difference between the handle releasing and the
/// retries starving and failing.
pub(super) async fn commit_swap_blocking(
    home: &std::path::Path,
    staging: &std::path::Path,
    vault_hash: &str,
    started_at: &str,
) -> Result<(), VaultError> {
    let (home, staging, vault_hash, started_at) = (
        home.to_owned(),
        staging.to_owned(),
        vault_hash.to_owned(),
        started_at.to_owned(),
    );
    tokio::task::spawn_blocking(move || {
        journal::commit_preserving(&home, &staging, &vault_hash, &started_at, &[SNAPSHOTS_DIR])
    })
    .await
    .map_err(|e| VaultError::Storage(StorageError::Io(format!("swap task join: {e}"))))?
}

/// Resolve + fully self-verify a snapshot: the manifest reads, its `format_version` is not
/// NEWER than we understand (H1 — refuse newer, accept older forever), the snapshot's
/// `vault.vdb` matches the manifest hash, and every referenced object is present and
/// BLAKE3-intact. Returns the resolved snapshot dir + its manifest.
///
/// **Read-only, no session.** Extracted (slice 5.2.3) so the seamless-revert PRE-FLIGHT can run
/// it while the live session is still alive: a bad-snapshot failure there leaves the user
/// unlocked instead of ejected. `revert_to_snapshot` re-runs it (cheap, idempotent) as its own
/// step 1.
pub(super) fn verify_snapshot_revertable(
    snapshots_dir: &std::path::Path,
    snapshot_id: &str,
) -> Result<(PathBuf, SnapshotManifest), VaultError> {
    let snap_dir = store::resolve_snapshot_dir(snapshots_dir, snapshot_id)
        .map_err(|_| VaultError::SnapshotNotFound(snapshot_id.to_owned()))?;
    let manifest = store::read_manifest(&snap_dir)
        .map_err(|e| VaultError::SnapshotManifestUnreadable(e.to_string()))?;
    // 🔴 H1 applies here too — refuse NEWER, accept OLDER. This shipped as `!=` in 5.2.1,
    // which would orphan every existing snapshot the day `SNAPSHOT_FORMAT_VERSION` becomes
    // 2. Same rule as the `.vbk` (`restore_vault`): once a store exists on a user's disk,
    // every future version must read it — forever. Pinned by a test; don't restore `!=`.
    if manifest.format_version > SNAPSHOT_FORMAT_VERSION {
        return Err(VaultError::SnapshotUnsupportedFormat(
            manifest.format_version,
        ));
    }
    let snap_vault = snap_dir.join(VAULT_FILE);
    let (_vsize, snap_vault_hash) = archive::hash_file(&snap_vault)
        .map_err(|_| VaultError::SnapshotCorrupt("snapshot vault.vdb is unreadable".into()))?;
    if snap_vault_hash != manifest.vault_blake3 {
        return Err(VaultError::SnapshotCorrupt(
            "snapshot vault.vdb failed verification".into(),
        ));
    }
    for obj in &manifest.objects {
        let obj_path = store::object_path(snapshots_dir, &obj.blake3);
        let (_s, hash) = archive::hash_file(&obj_path).map_err(|_| {
            VaultError::SnapshotCorrupt(format!("missing object b3-{}", obj.blake3))
        })?;
        if hash != obj.blake3 {
            return Err(VaultError::SnapshotCorrupt(format!(
                "object b3-{} failed verification",
                obj.blake3
            )));
        }
    }
    Ok((snap_dir, manifest))
}

#[instrument(skip_all, fields(vault = %input.vault.display(), snapshot = %input.snapshot_id))]
pub async fn revert_to_snapshot(
    repo_factory: &dyn VaultRepositoryFactory,
    keychain: &dyn KeychainProvider,
    input: RevertToSnapshotInput,
) -> Result<RevertReport, VaultError> {
    let home = input.vault;
    let snapshots_dir = home.join(SNAPSHOTS_DIR);

    // 0. Reconcile any prior interrupted restore of this home before a fresh swap.
    journal::recover_if_pending(&home)?;

    // 1. Resolve + verify the snapshot. Self-verifying: each object's BLAKE3 == its filename,
    //    and the snapshot's `vault.vdb` matches the manifest — no trust in the (maybe corrupt)
    //    live target is required.
    let (snap_dir, manifest) = verify_snapshot_revertable(&snapshots_dir, &input.snapshot_id)?;
    let snap_vault = snap_dir.join(VAULT_FILE);

    // 2. Rollback gate + ⑭ auto-snapshot, both gated on the target being READABLE. A `None`
    //    identity means a corrupt/missing target: nothing to roll back over, nothing to
    //    preserve — proceed straight to the revert (the H0 disaster path).
    if let Some(id) = read_target_identity(&home).await {
        if let Some(target_ctr) = id.commit_counter {
            if target_ctr > manifest.commit_counter && !input.confirm_rollback {
                return Err(VaultError::RollbackNotConfirmed);
            }
        }
        auto_snapshot_pre_restore(&home, &snapshots_dir, id.entry_count).await;
    }

    // 3. Stage the chosen snapshot as the new home: its `vault.vdb` + blobs reconstructed from
    //    the object pool + an EMPTY `snapshots/` (the preserving swap fills it from `.old`).
    let staging = journal::staging_path(&home);
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| io_ctx("clear stale staging", &e))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| io_ctx("create staging", &e))?;

    let staged_vault = journal::staged_vault(&staging);
    std::fs::copy(&snap_vault, &staged_vault).map_err(|e| io_ctx("stage snapshot vault", &e))?;
    journal::fsync_file(&staged_vault)?;

    let staged_blobs = journal::staged_blobs(&staging);
    std::fs::create_dir_all(&staged_blobs).map_err(|e| io_ctx("create staged blobs", &e))?;
    std::fs::create_dir_all(staging.join(SNAPSHOTS_DIR))
        .map_err(|e| io_ctx("create staged snapshots", &e))?;

    let mut blob_count: u64 = 0;
    for obj in &manifest.objects {
        let blob_name = format!("{}.blob", obj.entry_id);
        if !journal::is_safe_blob_name(&blob_name) {
            std::fs::remove_dir_all(&staging).ok();
            return Err(VaultError::SnapshotCorrupt(format!(
                "snapshot names an unsafe blob: {}",
                obj.entry_id
            )));
        }
        let src = store::object_path(&snapshots_dir, &obj.blake3);
        let dest = staged_blobs.join(&blob_name);
        std::fs::copy(&src, &dest).map_err(|e| io_ctx("reconstruct blob from object", &e))?;
        journal::fsync_file(&dest)?;
        blob_count = blob_count.saturating_add(1);
    }
    journal::fsync_dir(&staging);

    // Re-hash the staged vault — anchors the journal (what recovery re-verifies).
    let (_s, staged_hash) = archive::hash_file(&staged_vault)?;

    // 4. COMMIT: the write-ahead swap, PRESERVING `snapshots/` (§A / Decision ⑮), on a
    //    blocking thread so a lingering just-locked handle can release while it retries (M1).
    let started_at = format_rfc3339_millis(now());
    commit_swap_blocking(&home, &staging, &staged_hash, &started_at).await?;

    // 5. POST (L1): reopen (fatal — signals a bad revert) + one best-effort audit row + the
    //    best-effort rollback-mirror re-baseline. Nothing here may fail the revert.
    let repo = repo_factory.open(&home).await?;
    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::BackupRestored,
        occurred_at: now(),
        device_id: None,
    };
    if let Err(e) = repo.append_audit(&event).await {
        warn!(error = %e, "revert committed but the audit row could not be written");
    }
    rebaseline_rollback_mirror(repo.as_ref(), keychain).await;

    Ok(RevertReport {
        vault_uuid: manifest.vault_uuid.clone(),
        entry_count: manifest.entry_count,
        blob_count,
        reverted_at: started_at,
    })
}

/// Outcome of a seamless in-place revert (slice 5.2.3, Decision ⑰).
pub enum SeamlessRevertOutcome {
    /// The swap committed and the vault re-opened — the user stays inside it. The session is
    /// boxed: it is far larger than the other variants (a whole live `VaultSession`).
    Reverted {
        session: Box<VaultSession>,
        report: RevertReport,
    },
    /// The swap committed, but re-opening failed — a STALE snapshot whose ⑬ rewrap failed, so
    /// its `vault.vdb` no longer opens under the live KEK. The revert **succeeded**; the user
    /// re-unlocks with the credentials of that snapshot's moment. NOT a revert failure (L1).
    NeedsUnlock { report: RevertReport },
    /// The session was torn down but the swap itself FAILED (e.g. a full disk after the
    /// read-only preflight passed). The vault is unchanged and the revert did NOT happen — the
    /// user lands on the launch screen with an honest error. Rare.
    CommitFailed { error: VaultError },
}

/// Revert a snapshot IN PLACE while keeping the user inside the vault (slice 5.2.3, Decision ⑰).
///
/// The KEK accessor, `commit_preserving`, and `rename_retrying` are all crate-private, so this
/// orchestration must live in core. It:
///
/// 1. PRE-FLIGHTs the snapshot (read-only) with the session STILL ALIVE — a bad-snapshot error
///    returns here and the caller keeps the user unlocked (the ejection-bug fix).
/// 2. Only then clones the KEK, closes the session's DB (synchronous, Windows-safe handle
///    release), and runs the existing file-op [`revert_to_snapshot`] (whose swap uses
///    `rename_retrying` on a blocking thread and preserves `snapshots/`).
/// 3. Re-opens with `unlock_with_kek_quiet` (no forged audit row). A re-open failure is folded
///    into [`SeamlessRevertOutcome::NeedsUnlock`], never surfaced as a revert failure — the swap
///    already committed.
///
/// The caller normally passes `confirm_rollback = true` (reverting from inside IS the
/// confirmation); an unconfirmed rollback is caught in the read-only preflight (step 1) and
/// returned as `RollbackNotConfirmed` with the session untouched, so it can never tear down a
/// live session either.
pub async fn revert_to_snapshot_in_session(
    session: &VaultSession,
    unlock: &UnlockVault,
    repo_factory: &dyn VaultRepositoryFactory,
    keychain: &dyn KeychainProvider,
    input: RevertToSnapshotInput,
) -> Result<SeamlessRevertOutcome, VaultError> {
    let home = session.vault_id().path().to_path_buf();

    // 1. PRE-FLIGHT (read-only, session ALIVE): resolve + self-verify the snapshot, AND the
    //    rollback gate. Any refusal here returns with the session UNTOUCHED, so the caller keeps
    //    the user unlocked — the ejection-bug fix must cover EVERY pre-commit refusal, not just a
    //    bad snapshot. `revert_to_snapshot` re-checks both after teardown; the double-check is
    //    cheap and idempotent. The target read is `mode=ro` and coexists with the live session's
    //    connection (SQLite WAL allows concurrent readers).
    let (_snap_dir, manifest) =
        verify_snapshot_revertable(&home.join(SNAPSHOTS_DIR), &input.snapshot_id)?;
    if !input.confirm_rollback
        && let Some(id) = read_target_identity(&home).await
        && let Some(target_ctr) = id.commit_counter
        && target_ctr > manifest.commit_counter
    {
        return Err(VaultError::RollbackNotConfirmed);
    }

    // 2. Committed to the swap → tear the session down. Clone the KEK out (in-crate; the same
    //    deref-copy pattern as `change_password`), then release the DB handle SYNCHRONOUSLY so
    //    the directory swap does not race a lingering `.vdb` handle on Windows.
    let kek: Zeroizing<[u8; KEK_LEN]> = Zeroizing::new(*session.kek.expose());
    close_session_db(session).await;

    let report = match revert_to_snapshot(repo_factory, keychain, input).await {
        Ok(report) => report,
        // Post-teardown, pre-swap failure (rare — the snapshot was just verified). The vault is
        // unchanged; report honestly rather than as a successful revert.
        Err(error) => return Ok(SeamlessRevertOutcome::CommitFailed { error }),
    };

    // 3. Re-enter with the held KEK, suppressing the redundant unlock audit row. A stale
    //    snapshot whose rewrap failed cannot open under this KEK → NeedsUnlock, NOT an error.
    match unlock.unlock_with_kek_quiet(home, kek).await {
        Ok(session) => Ok(SeamlessRevertOutcome::Reverted {
            session: Box::new(session),
            report,
        }),
        Err(_) => Ok(SeamlessRevertOutcome::NeedsUnlock { report }),
    }
}
