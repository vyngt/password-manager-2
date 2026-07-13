//! Restore intent journal + crash recovery (slice 5.2b) — the filesystem transaction.
//!
//! Restore overwrites the live `.vdb` **and** its `<stem>.vedge_blobs/` sibling
//! with an archived snapshot. A crash at any instant must leave a consistent
//! `(vault, blobs)` **pair** — never a new DB beside old blobs, never a phantom
//! empty vault. This module is the transaction that guarantees it.
//!
//! Design: **write-ahead journaling** — persist the state we are *about* to enter
//! (fsync it) *before* its filesystem op — plus **idempotent, fs-probing recovery**
//! that re-drives forward from the recorded state. [`RestoreState::VaultAside`] is
//! the commit point: its durable presence means recovery rolls forward; a bare
//! [`RestoreState::Staged`] rolls back. The original vault is preserved as
//! `.old` / `.old-wal` until the very last step, so a lossless rollback is always
//! available at every forward crash window.
//!
//! `recover_if_pending` is wired as the first act of
//! `SqliteVaultRepositoryFactory::open`, the single chokepoint every vault open
//! passes through — so recovery completes *before* `SQLite` connects (which would
//! otherwise create a phantom empty vault at a missing path via `mode=rwc`).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive::hash_file;
use crate::infrastructure::backup::manifest::VAULT_MEMBER;
use crate::infrastructure::blob::derive_blob_root;

/// Subdirectory inside the staging dir that holds the extracted `blobs/`.
const STAGED_BLOBS: &str = "blobs";

/// The four durable states of a restore. Written write-ahead: the journal records
/// the state whose filesystem op is *about* to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreState {
    /// Verified + staged; nothing live touched. Recovery: delete staging, keep live.
    Staged,
    /// About to move (or has moved) the live `.vdb` aside to `.old`. Commit point:
    /// recovery from here rolls **forward**.
    VaultAside,
    /// About to move (or has moved) the live blob dir aside to `.old`. Roll forward.
    BlobsAside,
    /// About to swap (or has swapped) the staged trees into place. Roll forward.
    Swapped,
}

/// The on-disk intent journal (a sibling of the `.vdb`, JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreJournal {
    pub state: RestoreState,
    pub vault_path: PathBuf,
    pub staging_dir: PathBuf,
    /// Lowercase-hex `BLAKE3` of the staged `vault.vdb` — recovery re-verifies the
    /// snapshot against this before trusting it as the new live vault.
    pub vault_blake3: String,
    /// RFC-3339 millis, fixed-width (matches the house timestamp rule).
    pub started_at: String,
}

/// Fine-grained checkpoints inside [`commit_swap`], between which a crash may fall.
///
/// Production passes a no-op hook; the journal unit tests inject a hook that aborts
/// at a chosen checkpoint to reproduce every crash window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checkpoint {
    Staged,
    VaultAsideJournal,
    VaultMoved,
    WalMoved,
    BlobsAsideJournal,
    BlobsMoved,
    SwappedJournal,
    VaultSwapped,
    BlobsSwapped,
    OldCleaned,
}

// ---- path derivation --------------------------------------------------------

/// Append a literal suffix to a path (unlike `with_extension`, which replaces).
fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn file_stem_lossy(vault_path: &Path) -> String {
    vault_path
        .file_stem()
        .map_or_else(|| "vault".to_owned(), |s| s.to_string_lossy().into_owned())
}

fn parent_of(vault_path: &Path) -> &Path {
    vault_path.parent().unwrap_or_else(|| Path::new("."))
}

/// The intent-journal path for a vault: `<parent>/<stem>.restore.intent`.
///
/// Stem-keyed (not a fixed `restore.intent`) so several vaults sharing a directory
/// never collide on one journal — a deliberate hardening over the spec's fixed name.
pub(crate) fn journal_path(vault_path: &Path) -> PathBuf {
    let name = format!("{}.restore.intent", file_stem_lossy(vault_path));
    parent_of(vault_path).join(name)
}

/// The staging dir for a restore: `<parent>/.<stem>.restore-staging`. A plain named
/// dir (not a `TempDir`): a crash leaves it on disk for recovery to find, and it is
/// removed explicitly on the pre-commit path and by recovery.
pub(crate) fn staging_path(vault_path: &Path) -> PathBuf {
    let name = format!(".{}.restore-staging", file_stem_lossy(vault_path));
    parent_of(vault_path).join(name)
}

/// The staged snapshot path inside a staging dir (`<staging>/vault.vdb`).
pub(crate) fn staged_vault(staging_dir: &Path) -> PathBuf {
    staging_dir.join(VAULT_MEMBER)
}

/// The staged blob dir inside a staging dir (`<staging>/blobs`).
pub(crate) fn staged_blobs(staging_dir: &Path) -> PathBuf {
    staging_dir.join(STAGED_BLOBS)
}

/// Whether an extracted blob member name is a safe, flat filename (no path traversal).
///
/// Restore accepts an arbitrary user-chosen archive, so a crafted `blobs/../evil`
/// member must never escape the staging dir. (The `tar` crate already refuses `..` on
/// read/write; this is defense-in-depth on the name we join onto a real path.)
pub(crate) fn is_safe_blob_name(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains('/')
        && !rel.contains('\\')
        && !rel.contains("..")
        && !Path::new(rel).is_absolute()
}

// ---- durability primitives --------------------------------------------------

fn io_ctx(op: &str, e: &std::io::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("{op}: {e}")))
}

fn io_msg(msg: impl Into<String>) -> VaultError {
    VaultError::Storage(StorageError::Io(msg.into()))
}

/// fsync a directory's own metadata (POSIX). No-op on Windows, where a directory
/// cannot be opened for fsync without `unsafe`; there, durability rests on file
/// `sync_all` + rename ordering + the journal.
#[cfg(unix)]
pub(crate) fn fsync_dir(dir: &Path) {
    if let Ok(f) = File::open(dir) {
        f.sync_all().ok();
    }
}

#[cfg(not(unix))]
pub(crate) const fn fsync_dir(_dir: &Path) {}

/// Flush a file's bytes to disk. Opens for write so `FlushFileBuffers` succeeds on
/// Windows (a read-only handle cannot be fsynced there).
pub(crate) fn fsync_file(path: &Path) -> Result<(), VaultError> {
    let f = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| io_ctx("open for fsync", &e))?;
    f.sync_all().map_err(|e| io_ctx("fsync file", &e))
}

/// Write the journal durably: `<name>.tmp` → `sync_all` → atomic rename → dir fsync.
/// The journal is never observed torn.
fn persist(journal_path: &Path, journal: &RestoreJournal) -> Result<(), VaultError> {
    let bytes = serde_json::to_vec(journal)
        .map_err(|e| VaultError::Storage(StorageError::Serialization(format!("journal: {e}"))))?;
    let tmp = with_suffix(journal_path, ".tmp");
    {
        let mut f = File::create(&tmp).map_err(|e| io_ctx("create journal tmp", &e))?;
        f.write_all(&bytes)
            .map_err(|e| io_ctx("write journal", &e))?;
        f.sync_all().map_err(|e| io_ctx("sync journal", &e))?;
    }
    std::fs::rename(&tmp, journal_path).map_err(|e| io_ctx("rename journal", &e))?;
    if let Some(parent) = journal_path.parent() {
        fsync_dir(parent);
    }
    // Loud on purpose: the kill-mid-restore smoke reads these transitions.
    warn!(state = ?journal.state, vault = %journal.vault_path.display(), "restore journal transition");
    Ok(())
}

fn read_journal(journal_path: &Path) -> Result<RestoreJournal, VaultError> {
    let bytes = std::fs::read(journal_path).map_err(|e| io_ctx("read journal", &e))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| VaultError::Storage(StorageError::Serialization(format!("journal: {e}"))))
}

fn remove_if_exists(path: &Path) -> Result<(), VaultError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_ctx("remove file", &e)),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), VaultError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_ctx("remove dir", &e)),
    }
}

/// Idempotent rename — the heart of recovery. `src`/`dst` name a file or a dir.
///
/// - only `dst` exists  → already done, no-op
/// - only `src` exists  → rename
/// - neither exists     → error (a required member vanished → caller rolls back)
/// - both exist         → error (unreachable in the state-dispatched sequence; a
///   loud failure beats silently clobbering one of them)
fn move_if_needed(src: &Path, dst: &Path) -> Result<(), VaultError> {
    match (src.exists(), dst.exists()) {
        (false, true) => Ok(()),
        (true, false) => std::fs::rename(src, dst).map_err(|e| io_ctx("rename", &e)),
        (false, false) => Err(io_msg(format!(
            "restore: neither {} nor {} exists",
            src.display(),
            dst.display()
        ))),
        (true, true) => Err(io_msg(format!(
            "restore: both {} and {} exist",
            src.display(),
            dst.display()
        ))),
    }
}

// ---- the swap ---------------------------------------------------------------

struct SwapPaths {
    p: PathBuf,
    pwal: PathBuf,
    pshm: PathBuf,
    pold: PathBuf,
    poldwal: PathBuf,
    b: PathBuf,
    bold: PathBuf,
    sv: PathBuf,
    sb: PathBuf,
    journal: PathBuf,
}

impl SwapPaths {
    fn for_vault(vault_path: &Path, staging_dir: &Path) -> Self {
        let b = derive_blob_root(vault_path);
        Self {
            p: vault_path.to_owned(),
            pwal: with_suffix(vault_path, "-wal"),
            pshm: with_suffix(vault_path, "-shm"),
            pold: with_suffix(vault_path, ".old"),
            poldwal: with_suffix(vault_path, ".old-wal"),
            bold: with_suffix(&b, ".old"),
            b,
            sv: staged_vault(staging_dir),
            sb: staged_blobs(staging_dir),
            journal: journal_path(vault_path),
        }
    }
}

/// Move the live vault aside: `work.vdb` → `.old`, `work.vdb-wal` → `.old-wal`
/// (preserve, never delete — a stale `-wal` may hold committed txns needed on
/// rollback), and delete the pure-cache `-shm`. A no-op when the target is absent
/// (install-from-backup: there is no original to preserve).
fn op_vault_aside(paths: &SwapPaths) -> Result<(), VaultError> {
    if paths.p.exists() {
        move_if_needed(&paths.p, &paths.pold)?;
    }
    if paths.pwal.exists() {
        move_if_needed(&paths.pwal, &paths.poldwal)?;
    }
    remove_if_exists(&paths.pshm)
}

/// Move the live blob dir aside (single dir rename; atomic metadata op). Skipped
/// when the target has no blob dir.
fn op_blobs_aside(paths: &SwapPaths) -> Result<(), VaultError> {
    if paths.b.exists() {
        move_if_needed(&paths.b, &paths.bold)?;
    }
    Ok(())
}

/// Swap the staged snapshot + blobs into place.
fn op_swap_in(paths: &SwapPaths) -> Result<(), VaultError> {
    move_if_needed(&paths.sv, &paths.p)?;
    if paths.sb.exists() {
        move_if_needed(&paths.sb, &paths.b)?;
    }
    Ok(())
}

/// Delete the `.old` backups + the staging dir (the last thing before the journal).
fn op_cleanup(paths: &SwapPaths, staging_dir: &Path) -> Result<(), VaultError> {
    remove_if_exists(&paths.pold)?;
    remove_if_exists(&paths.poldwal)?;
    remove_dir_if_exists(&paths.bold)?;
    remove_dir_if_exists(staging_dir)
}

/// Production entry point: run the write-ahead swap with a no-op checkpoint hook.
pub(crate) fn commit(
    vault_path: &Path,
    staging_dir: &Path,
    vault_blake3: &str,
    started_at: &str,
) -> Result<(), VaultError> {
    let mut noop = |_cp: Checkpoint| -> Result<(), VaultError> { Ok(()) };
    commit_swap(vault_path, staging_dir, vault_blake3, started_at, &mut noop)
}

/// The write-ahead swap. Persists each state *before* the op it authorizes; the
/// original vault survives as `.old` until the final cleanup. `hook` fires at each
/// checkpoint (production: no-op; tests: abort to simulate a crash).
pub(crate) fn commit_swap(
    vault_path: &Path,
    staging_dir: &Path,
    vault_blake3: &str,
    started_at: &str,
    hook: &mut dyn FnMut(Checkpoint) -> Result<(), VaultError>,
) -> Result<(), VaultError> {
    let paths = SwapPaths::for_vault(vault_path, staging_dir);
    let make = |state| RestoreJournal {
        state,
        vault_path: vault_path.to_owned(),
        staging_dir: staging_dir.to_owned(),
        vault_blake3: vault_blake3.to_owned(),
        started_at: started_at.to_owned(),
    };

    persist(&paths.journal, &make(RestoreState::Staged))?;
    hook(Checkpoint::Staged)?;

    persist(&paths.journal, &make(RestoreState::VaultAside))?;
    hook(Checkpoint::VaultAsideJournal)?;
    if paths.p.exists() {
        move_if_needed(&paths.p, &paths.pold)?;
    }
    hook(Checkpoint::VaultMoved)?;
    if paths.pwal.exists() {
        move_if_needed(&paths.pwal, &paths.poldwal)?;
    }
    remove_if_exists(&paths.pshm)?;
    hook(Checkpoint::WalMoved)?;

    persist(&paths.journal, &make(RestoreState::BlobsAside))?;
    hook(Checkpoint::BlobsAsideJournal)?;
    op_blobs_aside(&paths)?;
    hook(Checkpoint::BlobsMoved)?;

    persist(&paths.journal, &make(RestoreState::Swapped))?;
    hook(Checkpoint::SwappedJournal)?;
    move_if_needed(&paths.sv, &paths.p)?;
    hook(Checkpoint::VaultSwapped)?;
    if paths.sb.exists() {
        move_if_needed(&paths.sb, &paths.b)?;
    }
    hook(Checkpoint::BlobsSwapped)?;

    op_cleanup(&paths, staging_dir)?;
    hook(Checkpoint::OldCleaned)?;

    remove_if_exists(&paths.journal)
}

// ---- recovery ---------------------------------------------------------------

/// Crash recovery, run before every vault open. Reconciles any pending restore to a
/// consistent `(vault, blobs)` pair, then returns the state it recovered from (or
/// `None` if nothing was pending).
///
/// Roll back at [`RestoreState::Staged`] (nothing destructive ran); roll forward
/// otherwise. On a corrupt staged snapshot it restores the `.old` original; if
/// neither a verified snapshot nor the `.old` original survives it returns `Err`
/// (so `open` aborts *before* `mode=rwc` could create an empty vault) and leaves the
/// journal for inspection.
pub(crate) fn recover_if_pending(vault_path: &Path) -> Result<Option<RestoreState>, VaultError> {
    let journal = journal_path(vault_path);
    if !journal.exists() {
        // No journal ⇒ live was never touched. Reap a pre-commit orphan staging dir.
        let staging = staging_path(vault_path);
        if staging.exists() {
            remove_dir_if_exists(&staging)?;
        }
        return Ok(None);
    }

    // The journal path is parent+stem-keyed (`journal_path`), so a journal found here
    // necessarily belongs to THIS vault. We deliberately do NOT byte-compare
    // `journal_data.vault_path` against `vault_path`: path representations legitimately
    // differ (Windows casing, 8.3 short names, relative vs absolute), and a false "not
    // ours" skip would let `mode=rwc` fabricate an empty vault over a pending restore.
    let journal_data = read_journal(&journal)?;

    match journal_data.state {
        RestoreState::Staged => {
            remove_dir_if_exists(&journal_data.staging_dir)?;
            remove_if_exists(&journal)?;
            Ok(Some(RestoreState::Staged))
        }
        forward => forward_or_rollback(&journal_data, forward),
    }
}

fn forward_or_rollback(
    journal: &RestoreJournal,
    state: RestoreState,
) -> Result<Option<RestoreState>, VaultError> {
    let paths = SwapPaths::for_vault(&journal.vault_path, &journal.staging_dir);

    // Re-verify the staged snapshot (still at `sv`, or already swapped to `p`). Any
    // doubt — missing file, hash mismatch, or an I/O error reading it — rolls back to
    // the preserved original rather than trusting a possibly-torn snapshot.
    //
    // NOTE: only the vault DB is re-verified; staged blobs carry no digest in the
    // journal, so a torn staged blob isn't caught here. That is not silent corruption
    // — each blob is AEAD-sealed, so a torn one reads as `DecryptionFailed` for that
    // one document, never wrong plaintext. Per-blob journal digests are a follow-up.
    let candidate = if paths.sv.exists() {
        paths.sv.clone()
    } else {
        paths.p.clone()
    };
    let good = candidate.exists()
        && matches!(hash_file(&candidate), Ok((_, ref h)) if *h == journal.vault_blake3);
    if !good {
        warn!(
            ?state,
            "restore snapshot failed re-verification — rolling back"
        );
        return roll_back_to_old(&paths, &journal.staging_dir, state);
    }

    // Roll forward from the recorded state, re-driving only the remaining ops.
    match state {
        RestoreState::VaultAside => {
            op_vault_aside(&paths)?;
            op_blobs_aside(&paths)?;
            op_swap_in(&paths)?;
        }
        RestoreState::BlobsAside => {
            op_blobs_aside(&paths)?;
            op_swap_in(&paths)?;
        }
        RestoreState::Swapped => {
            op_swap_in(&paths)?;
        }
        // `Staged` is handled by the caller and never reaches here; be defensive
        // rather than panic (the workspace denies `unreachable!`).
        RestoreState::Staged => {
            return Err(io_msg("internal: Staged reached the forward path"));
        }
    }

    op_cleanup(&paths, &journal.staging_dir)?;
    remove_if_exists(&paths.journal)?;
    Ok(Some(state))
}

/// Restore the `.old` original after a corrupt staged snapshot. Returns `Ok` once a
/// consistent original is back in place (the failed restore simply did not take
/// effect); `Err` only when there is no `.old` to restore either.
fn roll_back_to_old(
    paths: &SwapPaths,
    staging_dir: &Path,
    state: RestoreState,
) -> Result<Option<RestoreState>, VaultError> {
    if !paths.pold.exists() {
        // No `.old` to restore. If we're still at `VaultAside`, the destructive move
        // never ran (rename is atomic, and `.old` is absent), so the live `.vdb` IS
        // the intact original — keep it rather than lock the vault out.
        if state == RestoreState::VaultAside && paths.p.exists() {
            remove_dir_if_exists(staging_dir)?;
            remove_if_exists(&paths.journal)?;
            return Ok(Some(state));
        }
        // Otherwise nothing recoverable survives. Refuse to open (never fall through
        // to `mode=rwc`); keep the journal for inspection.
        return Err(io_msg(
            "restore interrupted and unrecoverable: neither a verified snapshot nor a backup of the original vault is present",
        ));
    }

    // Discard any half-written NEW vault, then swing the original back.
    remove_if_exists(&paths.p)?;
    move_if_needed(&paths.pold, &paths.p)?;
    if paths.poldwal.exists() {
        move_if_needed(&paths.poldwal, &paths.pwal)?;
    }
    if paths.bold.exists() {
        remove_dir_if_exists(&paths.b)?;
        move_if_needed(&paths.bold, &paths.b)?;
    } else if state == RestoreState::Swapped {
        // The original had no blob dir, but the forward swap already installed NEW
        // blobs at `b`; drop them so the rolled-back OLD vault is never paired with
        // NEW blobs (there is no `.old` blob dir to swing back).
        remove_dir_if_exists(&paths.b)?;
    }
    remove_dir_if_exists(staging_dir)?;
    remove_if_exists(&paths.journal)?;
    Ok(Some(state))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use
    )]

    use super::*;

    /// A scratch vault dir with a fake live `.vdb` (+ optional blob) and a staged
    /// snapshot, ready to drive `commit_swap` / `recover_if_pending`.
    struct Fixture {
        _dir: tempfile::TempDir,
        vault: PathBuf,
        staging: PathBuf,
        old_bytes: Vec<u8>,
        new_bytes: Vec<u8>,
        new_hash: String,
    }

    impl Fixture {
        fn new(with_live_blob: bool, with_staged_blob: bool) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let vault = dir.path().join("work.vdb");
            let old_bytes = b"OLD vault bytes -- the original".to_vec();
            std::fs::write(&vault, &old_bytes).unwrap();
            // A live WAL sidecar, as a real open would leave.
            std::fs::write(with_suffix(&vault, "-wal"), b"old-wal").unwrap();

            let blob_dir = derive_blob_root(&vault);
            if with_live_blob {
                std::fs::create_dir_all(&blob_dir).unwrap();
                std::fs::write(blob_dir.join("OLDENTRY.blob"), b"old blob").unwrap();
            }

            let staging = staging_path(&vault);
            std::fs::create_dir_all(&staging).unwrap();
            let new_bytes = b"NEW vault bytes -- from the backup snapshot".to_vec();
            let sv = staged_vault(&staging);
            std::fs::write(&sv, &new_bytes).unwrap();
            let (_, new_hash) = hash_file(&sv).unwrap();
            if with_staged_blob {
                let sb = staged_blobs(&staging);
                std::fs::create_dir_all(&sb).unwrap();
                std::fs::write(sb.join("NEWENTRY.blob"), b"new blob").unwrap();
            }

            Self {
                _dir: dir,
                vault,
                staging,
                old_bytes,
                new_bytes,
                new_hash,
            }
        }

        fn commit_until(&self, stop: Option<Checkpoint>) -> Result<(), VaultError> {
            let mut hook = |cp: Checkpoint| -> Result<(), VaultError> {
                if stop == Some(cp) {
                    Err(io_msg("injected crash"))
                } else {
                    Ok(())
                }
            };
            commit_swap(&self.vault, &self.staging, &self.new_hash, "t", &mut hook)
        }

        fn live_vault_bytes(&self) -> Vec<u8> {
            std::fs::read(&self.vault).unwrap()
        }

        /// No transient artifacts left behind. `-wal` is checked per-direction: a
        /// roll-forward leaves the fresh snapshot with no `-wal`; a roll-back
        /// restores the original's `-wal`, which must survive.
        fn assert_clean(&self, expect_new: bool) {
            assert!(!journal_path(&self.vault).exists(), "journal left behind");
            assert!(!self.staging.exists(), "staging left behind");
            assert!(
                !with_suffix(&self.vault, ".old").exists(),
                ".old left behind"
            );
            assert!(
                !with_suffix(&self.vault, ".old-wal").exists(),
                ".old-wal left behind"
            );
            assert!(
                !with_suffix(&derive_blob_root(&self.vault), ".old").exists(),
                "blobs .old left behind"
            );
            if expect_new {
                assert!(
                    !with_suffix(&self.vault, "-wal").exists(),
                    "stale -wal beside the new vault"
                );
            } else {
                assert!(
                    with_suffix(&self.vault, "-wal").exists(),
                    "original -wal must be preserved on roll-back"
                );
            }
        }
    }

    #[test]
    fn full_commit_lands_on_new_and_cleans_up() {
        let fx = Fixture::new(true, true);
        fx.commit_until(None).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes);
        let blob = derive_blob_root(&fx.vault).join("NEWENTRY.blob");
        assert!(blob.exists(), "new blob swapped in");
        assert!(
            !derive_blob_root(&fx.vault).join("OLDENTRY.blob").exists(),
            "old blob gone"
        );
        fx.assert_clean(true);
    }

    /// Crash at EVERY checkpoint → recovery lands on a consistent, clean pair.
    #[test]
    fn crash_at_every_checkpoint_recovers_consistently() {
        use Checkpoint::{
            BlobsAsideJournal, BlobsMoved, BlobsSwapped, OldCleaned, Staged, SwappedJournal,
            VaultAsideJournal, VaultMoved, VaultSwapped, WalMoved,
        };
        let checkpoints = [
            Staged,
            VaultAsideJournal,
            VaultMoved,
            WalMoved,
            BlobsAsideJournal,
            BlobsMoved,
            SwappedJournal,
            VaultSwapped,
            BlobsSwapped,
            OldCleaned,
        ];
        for cp in checkpoints {
            let fx = Fixture::new(true, true);
            // Injected crash aborts commit; the fs is left mid-transaction.
            let _ = fx.commit_until(Some(cp));
            let recovered = recover_if_pending(&fx.vault).unwrap();

            // Staged rolls back to OLD; every later checkpoint rolls forward to NEW.
            let expect_new = cp != Staged;
            let live = fx.live_vault_bytes();
            if expect_new {
                assert_eq!(live, fx.new_bytes, "checkpoint {cp:?} should be NEW");
                assert!(
                    derive_blob_root(&fx.vault).join("NEWENTRY.blob").exists(),
                    "checkpoint {cp:?}: new blob present"
                );
            } else {
                assert_eq!(live, fx.old_bytes, "checkpoint {cp:?} should be OLD");
                assert!(
                    derive_blob_root(&fx.vault).join("OLDENTRY.blob").exists(),
                    "checkpoint {cp:?}: old blob preserved"
                );
            }
            assert!(recovered.is_some(), "checkpoint {cp:?}: recovery ran");
            fx.assert_clean(expect_new);
            // Idempotent: a second pass is a no-op.
            assert!(recover_if_pending(&fx.vault).unwrap().is_none());
        }
    }

    #[test]
    fn no_journal_reaps_orphan_staging() {
        let fx = Fixture::new(false, false);
        // Staging exists but no journal ⇒ a pre-commit crash. Live untouched.
        assert!(recover_if_pending(&fx.vault).unwrap().is_none());
        assert!(!fx.staging.exists(), "orphan staging reaped");
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes);
    }

    #[test]
    fn corrupt_staged_snapshot_rolls_back_to_old() {
        let fx = Fixture::new(true, true);
        // Crash after the vault was moved aside, before the swap.
        let _ = fx.commit_until(Some(Checkpoint::BlobsMoved));
        // Corrupt the staged snapshot so its hash no longer matches the journal.
        std::fs::write(staged_vault(&fx.staging), b"corrupted staged bytes").unwrap();

        recover_if_pending(&fx.vault).unwrap();
        assert_eq!(
            fx.live_vault_bytes(),
            fx.old_bytes,
            "rolled back to the original"
        );
        assert!(
            derive_blob_root(&fx.vault).join("OLDENTRY.blob").exists(),
            "original blob restored"
        );
        fx.assert_clean(false);
    }

    #[test]
    fn unrecoverable_when_neither_new_nor_old_survives() {
        let fx = Fixture::new(true, true);
        let _ = fx.commit_until(Some(Checkpoint::BlobsMoved));
        // Destroy BOTH the staged snapshot and the .old original.
        std::fs::write(staged_vault(&fx.staging), b"corrupt").unwrap();
        std::fs::remove_file(with_suffix(&fx.vault, ".old")).unwrap();

        let err = recover_if_pending(&fx.vault).unwrap_err();
        assert!(format!("{err:?}").contains("unrecoverable"));
        // The journal is kept for inspection, and no empty vault is fabricated.
        assert!(journal_path(&fx.vault).exists(), "journal kept");
        assert!(!fx.vault.exists(), "no phantom vault created");
    }

    #[test]
    fn rejects_unsafe_blob_names() {
        assert!(is_safe_blob_name("01ARZ3NDEKTSV4RRFFQ69G5FAV.blob"));
        assert!(!is_safe_blob_name(""));
        assert!(!is_safe_blob_name("../evil"));
        assert!(!is_safe_blob_name("..\\evil"));
        assert!(!is_safe_blob_name("sub/dir.blob"));
        assert!(!is_safe_blob_name("a\\b.blob"));
    }

    #[test]
    fn backup_with_no_blobs_restores_consistently() {
        let fx = Fixture::new(false, false);
        fx.commit_until(None).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes);
        fx.assert_clean(true);
    }

    /// A leftover crashed restore is reconciled by recovery, so a *fresh* commit over
    /// the same vault does not collide with its `.old` artifacts (the cross-restore
    /// "both exist" wedge). `restore_vault` runs `recover_if_pending` for this reason.
    #[test]
    fn recovery_reconciles_a_leftover_before_a_fresh_commit() {
        let fx = Fixture::new(true, true);
        // Crash after the swap, before cleanup: leftover journal=Swapped + `.old`.
        let _ = fx.commit_until(Some(Checkpoint::BlobsSwapped));
        recover_if_pending(&fx.vault).unwrap();
        fx.assert_clean(true);
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes);

        // A fresh commit over the recovered vault now succeeds.
        let staging2 = staging_path(&fx.vault);
        std::fs::create_dir_all(&staging2).unwrap();
        let newer = b"NEWER vault bytes from a second backup".to_vec();
        std::fs::write(staged_vault(&staging2), &newer).unwrap();
        let (_, h2) = hash_file(&staged_vault(&staging2)).unwrap();
        commit(&fx.vault, &staging2, &h2, "t2").unwrap();
        assert_eq!(fx.live_vault_bytes(), newer);
        fx.assert_clean(true);
    }

    /// Crash right after the `VaultAside` journal (before the vault is moved aside)
    /// with an unverifiable staged snapshot: the live `.vdb` is the intact original
    /// and recovery must KEEP it, not lock the vault out for want of a `.old`.
    #[test]
    fn corrupt_staging_at_vault_aside_keeps_the_intact_original() {
        let fx = Fixture::new(true, true);
        let _ = fx.commit_until(Some(Checkpoint::VaultAsideJournal));
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes, "move hasn't run yet");
        std::fs::remove_dir_all(&fx.staging).unwrap();
        recover_if_pending(&fx.vault).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes);
        assert!(
            derive_blob_root(&fx.vault).join("OLDENTRY.blob").exists(),
            "original blob untouched"
        );
        fx.assert_clean(false);
    }

    /// Rollback when the original had NO blob dir but the forward swap already
    /// installed new blobs: the new blobs must be dropped so the restored OLD vault
    /// isn't paired with NEW blobs.
    #[test]
    fn rollback_drops_new_blobs_when_original_had_no_blob_dir() {
        let fx = Fixture::new(false, true);
        let _ = fx.commit_until(Some(Checkpoint::BlobsSwapped));
        std::fs::write(&fx.vault, b"corrupted new vault bytes").unwrap();
        recover_if_pending(&fx.vault).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes, "original restored");
        let blobs = derive_blob_root(&fx.vault);
        assert!(
            !blobs.exists() || std::fs::read_dir(&blobs).unwrap().next().is_none(),
            "new blobs dropped on rollback"
        );
        fx.assert_clean(false);
    }
}
