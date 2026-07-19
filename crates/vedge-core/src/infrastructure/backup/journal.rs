//! Restore intent journal + crash recovery (slice 5.2b; home swap in 5.2.0) — the
//! filesystem transaction.
//!
//! Restore replaces a live vault **home** (`<name>.vedge/` = `vault.vdb` + `blobs/` +
//! `snapshots/`) with a staged home extracted from an archive. Because a vault is now a
//! single directory, the swap is **one directory rename** — no separate `(vault, blobs)`
//! dance, no `-wal`/`-shm` juggling (those live inside the home and travel with it). A
//! crash at any instant must leave a consistent home — never a new `vault.vdb` beside old
//! blobs, never a phantom empty vault.
//!
//! Design (unchanged from 5.2b, only simpler): **write-ahead journaling** — persist the
//! state we are *about* to enter (fsync it) *before* its filesystem op — plus
//! **idempotent, fs-probing recovery** that re-drives forward from the recorded state.
//! [`RestoreState::HomeAside`] is the commit point: its durable presence means recovery
//! rolls forward; a bare [`RestoreState::Staged`] rolls back. The original home is
//! preserved as `<name>.vedge.old` until the very last step, so a lossless rollback is
//! always available at every forward crash window.
//!
//! `recover_if_pending` is wired as the first act of
//! `SqliteVaultRepositoryFactory::open`, the single chokepoint every vault open passes
//! through — so recovery completes *before* `SQLite` connects (which would otherwise
//! create a phantom empty vault at a missing path via `mode=rwc`).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::domain::shared::{BLOBS_DIR, StorageError, VAULT_FILE};
use crate::domain::vault::errors::VaultError;
use crate::infrastructure::backup::archive::hash_file;

/// The three durable states of a restore. Written write-ahead: the journal records the
/// state whose filesystem op is *about* to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestoreState {
    /// Verified + staged; nothing live touched. Recovery: delete staging, keep live.
    Staged,
    /// About to move (or has moved) the live home aside to `.old`. Commit point:
    /// recovery from here rolls **forward**.
    HomeAside,
    /// About to swap (or has swapped) the staged home into place. Roll forward.
    Swapped,
}

/// The on-disk intent journal (a sibling of the home, JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreJournal {
    pub state: RestoreState,
    /// The live vault home this restore targets.
    pub home_path: PathBuf,
    pub staging_dir: PathBuf,
    /// Lowercase-hex `BLAKE3` of the staged `vault.vdb` — recovery re-verifies the
    /// snapshot against this before trusting it as the new live vault.
    pub vault_blake3: String,
    /// RFC-3339 millis, fixed-width (matches the house timestamp rule).
    pub started_at: String,
    /// Subdirectories of the home that live INSIDE it but must SURVIVE the swap — moved
    /// from the `.old` original back into the new home after the swap-in, and reclaimed on
    /// rollback (slice 5.2.1, Decision ⑮: a `revert_to_snapshot` must never delete the
    /// `snapshots/` store it lives beside). Empty for a `.vbk` restore (whole-home replace).
    /// `#[serde(default)]` so a pre-5.2.1 in-flight journal (crash + upgrade) reads as empty
    /// ⇒ the original whole-home behaviour ⇒ safe.
    #[serde(default)]
    pub preserve_subdirs: Vec<String>,
}

/// Fine-grained checkpoints inside [`commit_swap`], between which a crash may fall.
///
/// Production passes a no-op hook; the journal unit tests inject a hook that aborts at a
/// chosen checkpoint to reproduce every crash window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Checkpoint {
    Staged,
    HomeAsideJournal,
    HomeMoved,
    SwappedJournal,
    HomeSwapped,
    SubdirsPreserved,
    OldCleaned,
}

// ---- path derivation --------------------------------------------------------

/// Append a literal suffix to a path (unlike `with_extension`, which replaces).
fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn home_name_lossy(home: &Path) -> String {
    home.file_name()
        .map_or_else(|| "vault".to_owned(), |s| s.to_string_lossy().into_owned())
}

fn parent_of(home: &Path) -> &Path {
    home.parent().unwrap_or_else(|| Path::new("."))
}

/// The intent-journal path for a home: `<parent>/.<name>.restore.intent`.
///
/// Name-keyed (not a fixed `restore.intent`) so several homes sharing a directory never
/// collide on one journal — a deliberate hardening over the spec's fixed name.
pub(crate) fn journal_path(home: &Path) -> PathBuf {
    let name = format!(".{}.restore.intent", home_name_lossy(home));
    parent_of(home).join(name)
}

/// The staging dir for a restore: `<parent>/.<name>.restore-staging`. A plain named dir
/// (not a `TempDir`): a crash leaves it on disk for recovery to find, and it is removed
/// explicitly on the pre-commit path and by recovery. Same parent as the home ⇒ the
/// staged→live rename is always intra-volume (atomic).
pub(crate) fn staging_path(home: &Path) -> PathBuf {
    let name = format!(".{}.restore-staging", home_name_lossy(home));
    parent_of(home).join(name)
}

/// The staged `vault.vdb` inside a staging (staged-home) dir.
pub(crate) fn staged_vault(staging_dir: &Path) -> PathBuf {
    staging_dir.join(VAULT_FILE)
}

/// The staged `blobs/` dir inside a staging (staged-home) dir.
pub(crate) fn staged_blobs(staging_dir: &Path) -> PathBuf {
    staging_dir.join(BLOBS_DIR)
}

/// The live `vault.vdb` inside a home — recovery's re-verification candidate once the
/// staged home has already been swapped in.
fn home_vault(home: &Path) -> PathBuf {
    home.join(VAULT_FILE)
}

/// Whether an extracted blob member name is a safe, flat filename (no path traversal).
///
/// Restore accepts an arbitrary user-chosen archive, so a crafted `blobs/../evil` member
/// must never escape the staging dir. (The `tar` crate already refuses `..` on
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

/// fsync a directory's own metadata (POSIX). No-op on Windows, where a directory cannot
/// be opened for fsync without `unsafe`; there, durability rests on file `sync_all` +
/// rename ordering + the journal.
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

/// Write the journal durably: `<name>.tmp` → `sync_all` → atomic rename → dir fsync. The
/// journal is never observed torn.
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
    warn!(state = ?journal.state, home = %journal.home_path.display(), "restore journal transition");
    #[cfg(debug_assertions)]
    maybe_pause_for_kill_smoke(journal.state);
    Ok(())
}

/// Debug-only, env-gated pause for the on-device kill-mid-restore smoke (slice 5.9 ①).
///
/// The post-commit journal transitions are size-independent microsecond metadata renames, so a
/// hard `taskkill` can never land on one by timing alone. Set `VEDGE_RESTORE_PAUSE` to a state
/// name (`Staged` / `HomeAside` / `Swapped`, or `*` for every one) and this prints a
/// machine-readable `PAUSED:<state>` to stderr and blocks, giving a harness (or a human) a
/// deterministic window to kill the process exactly after that state became durable on disk.
///
/// 🔴 `cfg(debug_assertions)` **and** env-gated — release-impossible, exactly like the
/// `VEDGE_E2E_*` seams and the debug fast-KDF (`kdf_params::fast`). A pause reachable in a release
/// build would be a denial-of-service on every restore/revert/re-key.
#[cfg(debug_assertions)]
fn maybe_pause_for_kill_smoke(state: RestoreState) {
    let Some(want) = std::env::var_os("VEDGE_RESTORE_PAUSE") else {
        return;
    };
    let want = want.to_string_lossy();
    let name = format!("{state:?}");
    if want != "*" && want != name {
        return;
    }
    // The marker the harness greps for, then a generous window to be hard-killed. `persist` runs on
    // a blocking thread (`commit_swap` is called via `spawn_blocking`), so blocking it is safe.
    eprintln!("PAUSED:{name}");
    std::thread::sleep(std::time::Duration::from_secs(120));
    eprintln!("RESUMED:{name}");
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
/// - both exist         → error (unreachable in the state-dispatched sequence; a loud
///   failure beats silently clobbering one of them)
fn move_if_needed(src: &Path, dst: &Path) -> Result<(), VaultError> {
    match (src.exists(), dst.exists()) {
        (false, true) => Ok(()),
        (true, false) => rename_retrying(src, dst),
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

/// True for a Windows `ERROR_SHARING_VIOLATION` (32) — the OS refusing to rename a file that
/// still has a live handle. `PermissionDenied` is its `ErrorKind` mapping.
fn is_sharing_violation(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(32) || e.kind() == std::io::ErrorKind::PermissionDenied
}

/// Rename with a blocking retry on a Windows sharing/access violation (M1).
///
/// `sqlx`'s connection-pool close is NOT synchronous on `Drop`: after a vault is LOCKED, its
/// just-dropped session pool keeps the `.vdb` (+ `-wal`/`-shm`) handle open for a short while
/// before Windows releases it — longer for a vault carrying large attachments. Windows then
/// refuses to rename the home directory (`ERROR_SHARING_VIOLATION` 32 or `ERROR_ACCESS_DENIED`
/// 5). An in-place `revert_to_snapshot` runs right after a lock, so it hits this window; the
/// handle DOES release, it just needs time. Retry for up to ~10s. Callers run the swap on a
/// blocking thread (`spawn_blocking`) so the async runtime stays free to finish the pool close
/// while we wait. Non-contention errors return immediately (no point retrying a real failure).
pub(crate) fn rename_retrying(src: &Path, dst: &Path) -> Result<(), VaultError> {
    const ATTEMPTS: u32 = 100;
    const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
    let mut attempt: u32 = 0;
    loop {
        match std::fs::rename(src, dst) {
            Ok(()) => {
                if attempt > 0 {
                    warn!(
                        attempts = attempt,
                        "rename succeeded after retrying a locked handle"
                    );
                }
                return Ok(());
            }
            Err(e) if is_sharing_violation(&e) && attempt < ATTEMPTS.saturating_sub(1) => {
                attempt = attempt.saturating_add(1);
                std::thread::sleep(DELAY);
            }
            Err(e) => return Err(io_ctx("rename", &e)),
        }
    }
}

/// Recursively remove a directory, retrying on a Windows sharing/access violation (slice 5.2.4).
///
/// The same lingering-handle window as [`rename_retrying`]: a just-locked vault's `sqlx` pool can
/// hold the `.vdb` handle briefly, so `remove_dir_all` on the home can fail with
/// `ERROR_SHARING_VIOLATION` (32) / `ERROR_ACCESS_DENIED` (5) even though the handle will release.
/// A missing directory is success (idempotent — a crash-resume re-run finds it already gone).
/// Callers run this on a blocking thread so the async runtime stays free while we wait.
pub(crate) fn remove_dir_all_retrying(path: &Path) -> Result<(), VaultError> {
    const ATTEMPTS: u32 = 100;
    const DELAY: std::time::Duration = std::time::Duration::from_millis(100);
    let mut attempt: u32 = 0;
    loop {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) if is_sharing_violation(&e) && attempt < ATTEMPTS.saturating_sub(1) => {
                attempt = attempt.saturating_add(1);
                std::thread::sleep(DELAY);
            }
            Err(e) => return Err(io_ctx("remove dir", &e)),
        }
    }
}

// ---- the swap ---------------------------------------------------------------

struct SwapPaths {
    /// The live home (`<parent>/<name>.vedge`).
    home: PathBuf,
    /// The aside copy of the original home (`<name>.vedge.old`).
    old: PathBuf,
    /// The staged home (`<parent>/.<name>.restore-staging`).
    staging: PathBuf,
    journal: PathBuf,
}

impl SwapPaths {
    fn for_home(home: &Path, staging_dir: &Path) -> Self {
        Self {
            home: home.to_owned(),
            old: with_suffix(home, ".old"),
            staging: staging_dir.to_owned(),
            journal: journal_path(home),
        }
    }
}

/// Move the live home aside to `.old` (single dir rename; atomic metadata op). A no-op
/// when the target home is absent (install-from-backup: there is no original to preserve).
fn op_home_aside(paths: &SwapPaths) -> Result<(), VaultError> {
    if paths.home.exists() {
        move_if_needed(&paths.home, &paths.old)?;
    }
    Ok(())
}

/// Swap the staged home into place (single dir rename).
fn op_swap_in(paths: &SwapPaths) -> Result<(), VaultError> {
    move_if_needed(&paths.staging, &paths.home)
}

/// Move each preserved subdir from the `.old` original back into the new home (slice 5.2.1,
/// Decision ⑮). A `revert_to_snapshot` replaces the home from a snapshot but the `snapshots/`
/// store lives INSIDE the home — without this it would be swept away with `.old` at cleanup,
/// destroying the very safety net (incl. the just-taken `pre-restore` auto-snapshot).
///
/// Idempotent, keyed on `.old/<sub>` still existing: not-yet-run moves it; half-run
/// (`home/<sub>` absent) completes the move; fully-run (`.old/<sub>` gone) is a no-op. The
/// staged home's own copy of the subdir is dropped first so the live one can swing in.
fn op_preserve_subdirs(paths: &SwapPaths, subs: &[String]) -> Result<(), VaultError> {
    for sub in subs {
        let from = paths.old.join(sub);
        let to = paths.home.join(sub);
        if from.exists() {
            remove_dir_if_exists(&to)?;
            move_if_needed(&from, &to)?;
        }
    }
    Ok(())
}

/// Delete the `.old` original + the staging dir (the last thing before the journal).
fn op_cleanup(paths: &SwapPaths, staging_dir: &Path) -> Result<(), VaultError> {
    remove_dir_if_exists(&paths.old)?;
    remove_dir_if_exists(staging_dir)
}

/// Production entry point: swap `staging_dir` in as the home, PRESERVING the named subdirs
/// across the swap (they are carried over from the `.old` original rather than replaced).
///
/// 🔴 **There is deliberately no bare `commit(…)` convenience wrapper**, and adding one back
/// would be a mistake. Until slice 5.2.2 there was one — it called this with an empty
/// preserve-list — and `restore_vault` used it. That was a **latent data-loss bug**: since
/// 5.2.1 the snapshot store lives at `<home>/snapshots`, *inside* the very home being swapped,
/// so a whole-home swap **deleted every snapshot the vault had**, including the `pre-restore`
/// undo point taken moments earlier to make the operation reversible. It was never caught
/// because nothing tested for the absence of a directory nobody thought about.
///
/// Both destructive verbs (`revert_to_snapshot`, `replace_vault_from_backup`) now pass
/// `&[SNAPSHOTS_DIR]`. Requiring the argument means a future caller has to *decide* what
/// survives its swap, out loud, instead of inheriting a silent default that eats the safety
/// net. If you genuinely want to replace the entire home, pass `&[]` — and say why.
pub(crate) fn commit_preserving(
    home: &Path,
    staging_dir: &Path,
    vault_blake3: &str,
    started_at: &str,
    preserve_subdirs: &[&str],
) -> Result<(), VaultError> {
    let mut noop = |_cp: Checkpoint| -> Result<(), VaultError> { Ok(()) };
    commit_swap(
        home,
        staging_dir,
        vault_blake3,
        started_at,
        preserve_subdirs,
        &mut noop,
    )
}

/// The write-ahead swap. Persists each state *before* the op it authorizes; the original
/// home survives as `.old` until the final cleanup. `hook` fires at each checkpoint
/// (production: no-op; tests: abort to simulate a crash).
pub(crate) fn commit_swap(
    home: &Path,
    staging_dir: &Path,
    vault_blake3: &str,
    started_at: &str,
    preserve_subdirs: &[&str],
    hook: &mut dyn FnMut(Checkpoint) -> Result<(), VaultError>,
) -> Result<(), VaultError> {
    let paths = SwapPaths::for_home(home, staging_dir);
    let subs_owned: Vec<String> = preserve_subdirs.iter().map(|s| (*s).to_owned()).collect();
    let make = |state| RestoreJournal {
        state,
        home_path: home.to_owned(),
        staging_dir: staging_dir.to_owned(),
        vault_blake3: vault_blake3.to_owned(),
        started_at: started_at.to_owned(),
        preserve_subdirs: subs_owned.clone(),
    };

    persist(&paths.journal, &make(RestoreState::Staged))?;
    hook(Checkpoint::Staged)?;

    persist(&paths.journal, &make(RestoreState::HomeAside))?;
    hook(Checkpoint::HomeAsideJournal)?;
    op_home_aside(&paths)?;
    hook(Checkpoint::HomeMoved)?;

    persist(&paths.journal, &make(RestoreState::Swapped))?;
    hook(Checkpoint::SwappedJournal)?;
    op_swap_in(&paths)?;
    hook(Checkpoint::HomeSwapped)?;

    op_preserve_subdirs(&paths, &subs_owned)?;
    hook(Checkpoint::SubdirsPreserved)?;

    op_cleanup(&paths, staging_dir)?;
    hook(Checkpoint::OldCleaned)?;

    remove_if_exists(&paths.journal)
}

// ---- recovery ---------------------------------------------------------------

/// Crash recovery, run before every vault open. Reconciles any pending restore to a
/// consistent home, then returns the state it recovered from (or `None` if nothing was
/// pending).
///
/// Roll back at [`RestoreState::Staged`] (nothing destructive ran); roll forward
/// otherwise. On a corrupt staged snapshot it restores the `.old` original; if neither a
/// verified snapshot nor the `.old` original survives it returns `Err` (so `open` aborts
/// *before* `mode=rwc` could create an empty vault) and leaves the journal for inspection.
pub(crate) fn recover_if_pending(home: &Path) -> Result<Option<RestoreState>, VaultError> {
    let journal = journal_path(home);
    if !journal.exists() {
        // No journal ⇒ live was never touched. Reap a pre-commit orphan staging dir.
        let staging = staging_path(home);
        if staging.exists() {
            remove_dir_if_exists(&staging)?;
        }
        return Ok(None);
    }

    // The journal path is parent+name-keyed (`journal_path`), so a journal found here
    // necessarily belongs to THIS home. We deliberately do NOT byte-compare
    // `journal_data.home_path` against `home`: path representations legitimately differ
    // (Windows casing, 8.3 short names, relative vs absolute), and a false "not ours" skip
    // would let `mode=rwc` fabricate an empty vault over a pending restore.
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
    let paths = SwapPaths::for_home(&journal.home_path, &journal.staging_dir);

    // Re-verify the staged snapshot's `vault.vdb` (still in staging, or already swapped
    // into the home). Any doubt — missing file, hash mismatch, or an I/O error reading it
    // — rolls back to the preserved original rather than trusting a possibly-torn snapshot.
    //
    // NOTE: only the vault DB is re-verified; staged blobs carry no digest in the journal,
    // so a torn staged blob isn't caught here. That is not silent corruption — each blob is
    // AEAD-sealed, so a torn one reads as `DecryptionFailed` for that one document, never
    // wrong plaintext. Per-blob journal digests are a follow-up.
    let candidate = if paths.staging.exists() {
        staged_vault(&paths.staging)
    } else {
        home_vault(&paths.home)
    };
    let good = candidate.exists()
        && matches!(hash_file(&candidate), Ok((_, ref h)) if *h == journal.vault_blake3);
    if !good {
        warn!(
            ?state,
            "restore snapshot failed re-verification — rolling back"
        );
        return roll_back_to_old(
            &paths,
            &journal.staging_dir,
            &journal.preserve_subdirs,
            state,
        );
    }

    // Roll forward from the recorded state, re-driving only the remaining ops.
    match state {
        RestoreState::HomeAside => {
            op_home_aside(&paths)?;
            op_swap_in(&paths)?;
        }
        RestoreState::Swapped => {
            op_swap_in(&paths)?;
        }
        // `Staged` is handled by the caller and never reaches here; be defensive rather
        // than panic (the workspace denies `unreachable!`).
        RestoreState::Staged => {
            return Err(io_msg("internal: Staged reached the forward path"));
        }
    }

    // Preserve subdirs (slice 5.2.1) BEFORE cleanup deletes `.old`. Idempotent, so re-driving
    // a partially-preserved swap is safe.
    op_preserve_subdirs(&paths, &journal.preserve_subdirs)?;
    op_cleanup(&paths, &journal.staging_dir)?;
    remove_if_exists(&paths.journal)?;
    Ok(Some(state))
}

/// Restore the `.old` original after a corrupt staged snapshot. Returns `Ok` once a
/// consistent original is back in place (the failed restore simply did not take effect);
/// `Err` only when there is no `.old` to restore either.
fn roll_back_to_old(
    paths: &SwapPaths,
    staging_dir: &Path,
    preserve_subdirs: &[String],
    state: RestoreState,
) -> Result<Option<RestoreState>, VaultError> {
    if !paths.old.exists() {
        // No `.old` to restore. If we're still at `HomeAside`, the destructive move never
        // ran (rename is atomic, and `.old` is absent), so the live home IS the intact
        // original — keep it rather than lock the vault out.
        if state == RestoreState::HomeAside && paths.home.exists() {
            remove_dir_if_exists(staging_dir)?;
            remove_if_exists(&paths.journal)?;
            return Ok(Some(state));
        }
        // Otherwise nothing recoverable survives. Refuse to open (never fall through to
        // `mode=rwc`); keep the journal for inspection.
        return Err(io_msg(
            "restore interrupted and unrecoverable: neither a verified snapshot nor a backup of the original vault is present",
        ));
    }

    // 🔴 Symmetric reclaim (slice 5.2.1): if a preserved subdir was already moved OUT of
    // `.old` into the NEW home (a crash after `op_preserve_subdirs` began), swing it back
    // into `.old` before discarding the new home — otherwise a rollback would delete the
    // `snapshots/` store the revert was meant to keep. Keyed on `.old/<sub>` being absent.
    for sub in preserve_subdirs {
        let in_home = paths.home.join(sub);
        let in_old = paths.old.join(sub);
        if !in_old.exists() && in_home.exists() {
            move_if_needed(&in_home, &in_old)?;
        }
    }

    // Discard any half-installed NEW home (blobs and all — they moved in as one unit), then
    // swing the original back.
    remove_dir_if_exists(&paths.home)?;
    move_if_needed(&paths.old, &paths.home)?;
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

    /// A scratch vault home with a fake live `vault.vdb` (+ `-wal`, + optional blob) and a
    /// staged home, ready to drive `commit_swap` / `recover_if_pending`.
    struct Fixture {
        _dir: tempfile::TempDir,
        home: PathBuf,
        staging: PathBuf,
        old_bytes: Vec<u8>,
        new_bytes: Vec<u8>,
        new_hash: String,
    }

    impl Fixture {
        fn new(with_live_blob: bool, with_staged_blob: bool) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let home = dir.path().join("work.vedge");
            std::fs::create_dir_all(home.join(BLOBS_DIR)).unwrap();
            std::fs::create_dir_all(home.join("snapshots")).unwrap();
            let old_bytes = b"OLD vault bytes -- the original".to_vec();
            std::fs::write(home_vault(&home), &old_bytes).unwrap();
            // A live WAL sidecar INSIDE the home, as a real open would leave.
            std::fs::write(with_suffix(&home_vault(&home), "-wal"), b"old-wal").unwrap();
            if with_live_blob {
                std::fs::write(home.join(BLOBS_DIR).join("OLDENTRY.blob"), b"old blob").unwrap();
            }

            let staging = staging_path(&home);
            std::fs::create_dir_all(staged_blobs(&staging)).unwrap();
            std::fs::create_dir_all(staging.join("snapshots")).unwrap();
            let new_bytes = b"NEW vault bytes -- from the backup snapshot".to_vec();
            std::fs::write(staged_vault(&staging), &new_bytes).unwrap();
            let (_, new_hash) = hash_file(&staged_vault(&staging)).unwrap();
            if with_staged_blob {
                std::fs::write(staged_blobs(&staging).join("NEWENTRY.blob"), b"new blob").unwrap();
            }

            Self {
                _dir: dir,
                home,
                staging,
                old_bytes,
                new_bytes,
                new_hash,
            }
        }

        fn commit_until(&self, stop: Option<Checkpoint>) -> Result<(), VaultError> {
            self.commit_until_preserving(stop, &[])
        }

        fn commit_until_preserving(
            &self,
            stop: Option<Checkpoint>,
            preserve: &[&str],
        ) -> Result<(), VaultError> {
            let mut hook = |cp: Checkpoint| -> Result<(), VaultError> {
                if stop == Some(cp) {
                    Err(io_msg("injected crash"))
                } else {
                    Ok(())
                }
            };
            commit_swap(
                &self.home,
                &self.staging,
                &self.new_hash,
                "t",
                preserve,
                &mut hook,
            )
        }

        fn live_vault_bytes(&self) -> Vec<u8> {
            std::fs::read(home_vault(&self.home)).unwrap()
        }

        /// Seed a distinct marker inside the LIVE home's `snapshots/` (the store a revert must
        /// preserve). The staged home's `snapshots/` is left empty, so a correct preserve
        /// leaves the marker present in the reverted home.
        fn seed_live_snapshot(&self) {
            let dir = self.home.join("snapshots").join("pre-restore");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("marker"), b"the undo point").unwrap();
        }

        fn snapshot_marker_present(&self) -> bool {
            self.home
                .join("snapshots")
                .join("pre-restore")
                .join("marker")
                .is_file()
        }

        fn blob(&self, name: &str) -> PathBuf {
            self.home.join(BLOBS_DIR).join(name)
        }

        /// No transient artifacts left behind. The home's own `-wal` is checked
        /// per-direction: a roll-forward installs the fresh snapshot (no `-wal`); a
        /// roll-back swings the original home back, `-wal` and all.
        fn assert_clean(&self, expect_new: bool) {
            assert!(!journal_path(&self.home).exists(), "journal left behind");
            assert!(!self.staging.exists(), "staging left behind");
            assert!(
                !with_suffix(&self.home, ".old").exists(),
                ".old left behind"
            );
            let wal = with_suffix(&home_vault(&self.home), "-wal");
            if expect_new {
                assert!(!wal.exists(), "stale -wal in the new home");
            } else {
                assert!(wal.exists(), "original -wal must be preserved on roll-back");
            }
        }
    }

    #[test]
    fn full_commit_lands_on_new_and_cleans_up() {
        let fx = Fixture::new(true, true);
        fx.commit_until(None).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes);
        assert!(fx.blob("NEWENTRY.blob").exists(), "new blob swapped in");
        assert!(!fx.blob("OLDENTRY.blob").exists(), "old blob gone");
        fx.assert_clean(true);
    }

    /// Crash at EVERY checkpoint → recovery lands on a consistent, clean home.
    #[test]
    fn crash_at_every_checkpoint_recovers_consistently() {
        use Checkpoint::{
            HomeAsideJournal, HomeMoved, HomeSwapped, OldCleaned, Staged, SubdirsPreserved,
            SwappedJournal,
        };
        let checkpoints = [
            Staged,
            HomeAsideJournal,
            HomeMoved,
            SwappedJournal,
            HomeSwapped,
            SubdirsPreserved,
            OldCleaned,
        ];
        for cp in checkpoints {
            let fx = Fixture::new(true, true);
            // Injected crash aborts commit; the fs is left mid-transaction.
            let _ = fx.commit_until(Some(cp));
            let recovered = recover_if_pending(&fx.home).unwrap();

            // Staged rolls back to OLD; every later checkpoint rolls forward to NEW.
            let expect_new = cp != Staged;
            let live = fx.live_vault_bytes();
            if expect_new {
                assert_eq!(live, fx.new_bytes, "checkpoint {cp:?} should be NEW");
                assert!(
                    fx.blob("NEWENTRY.blob").exists(),
                    "checkpoint {cp:?}: new blob present"
                );
            } else {
                assert_eq!(live, fx.old_bytes, "checkpoint {cp:?} should be OLD");
                assert!(
                    fx.blob("OLDENTRY.blob").exists(),
                    "checkpoint {cp:?}: old blob preserved"
                );
            }
            assert!(recovered.is_some(), "checkpoint {cp:?}: recovery ran");
            fx.assert_clean(expect_new);
            // Idempotent: a second pass is a no-op.
            assert!(recover_if_pending(&fx.home).unwrap().is_none());
        }
    }

    #[test]
    fn no_journal_reaps_orphan_staging() {
        let fx = Fixture::new(false, false);
        // Staging exists but no journal ⇒ a pre-commit crash. Live untouched.
        assert!(recover_if_pending(&fx.home).unwrap().is_none());
        assert!(!fx.staging.exists(), "orphan staging reaped");
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes);
    }

    #[test]
    fn corrupt_staged_snapshot_rolls_back_to_old() {
        let fx = Fixture::new(true, true);
        // Crash after the home was moved aside, before the swap.
        let _ = fx.commit_until(Some(Checkpoint::HomeMoved));
        // Corrupt the staged snapshot so its hash no longer matches the journal.
        std::fs::write(staged_vault(&fx.staging), b"corrupted staged bytes").unwrap();

        recover_if_pending(&fx.home).unwrap();
        assert_eq!(
            fx.live_vault_bytes(),
            fx.old_bytes,
            "rolled back to the original"
        );
        assert!(fx.blob("OLDENTRY.blob").exists(), "original blob restored");
        fx.assert_clean(false);
    }

    #[test]
    fn unrecoverable_when_neither_new_nor_old_survives() {
        let fx = Fixture::new(true, true);
        let _ = fx.commit_until(Some(Checkpoint::HomeMoved));
        // Destroy BOTH the staged snapshot and the `.old` original.
        std::fs::write(staged_vault(&fx.staging), b"corrupt").unwrap();
        std::fs::remove_dir_all(with_suffix(&fx.home, ".old")).unwrap();

        let err = recover_if_pending(&fx.home).unwrap_err();
        assert!(format!("{err:?}").contains("unrecoverable"));
        // The journal is kept for inspection, and no empty vault is fabricated.
        assert!(journal_path(&fx.home).exists(), "journal kept");
        assert!(!fx.home.exists(), "no phantom home created");
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
        assert!(
            fx.home.join(BLOBS_DIR).is_dir(),
            "home keeps an (empty) blobs/"
        );
        fx.assert_clean(true);
    }

    /// A leftover crashed restore is reconciled by recovery, so a *fresh* commit over the
    /// same home does not collide with its `.old` artifacts (the cross-restore "both exist"
    /// wedge). `restore_vault` runs `recover_if_pending` for this reason.
    #[test]
    fn recovery_reconciles_a_leftover_before_a_fresh_commit() {
        let fx = Fixture::new(true, true);
        // Crash after the swap, before cleanup: leftover journal=Swapped + `.old`.
        let _ = fx.commit_until(Some(Checkpoint::HomeSwapped));
        recover_if_pending(&fx.home).unwrap();
        fx.assert_clean(true);
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes);

        // A fresh commit over the recovered home now succeeds.
        let staging2 = staging_path(&fx.home);
        std::fs::create_dir_all(staged_blobs(&staging2)).unwrap();
        std::fs::create_dir_all(staging2.join("snapshots")).unwrap();
        let newer = b"NEWER vault bytes from a second backup".to_vec();
        std::fs::write(staged_vault(&staging2), &newer).unwrap();
        let (_, h2) = hash_file(&staged_vault(&staging2)).unwrap();
        commit_preserving(&fx.home, &staging2, &h2, "t2", &["snapshots"]).unwrap();
        assert_eq!(fx.live_vault_bytes(), newer);
        fx.assert_clean(true);
    }

    /// Crash right after the `HomeAside` journal (before the home is moved aside) with an
    /// unverifiable staged snapshot: the live home is the intact original and recovery must
    /// KEEP it, not lock the vault out for want of a `.old`.
    #[test]
    fn corrupt_staging_at_home_aside_keeps_the_intact_original() {
        let fx = Fixture::new(true, true);
        let _ = fx.commit_until(Some(Checkpoint::HomeAsideJournal));
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes, "move hasn't run yet");
        std::fs::remove_dir_all(&fx.staging).unwrap();
        recover_if_pending(&fx.home).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes);
        assert!(fx.blob("OLDENTRY.blob").exists(), "original blob untouched");
        fx.assert_clean(false);
    }

    /// Rollback after a full swap drops the NEW home entirely (blobs and all) and swings
    /// the original back — the restored OLD vault is never paired with NEW blobs.
    #[test]
    fn rollback_drops_new_blobs_on_full_swap() {
        let fx = Fixture::new(false, true);
        let _ = fx.commit_until(Some(Checkpoint::HomeSwapped));
        // Corrupt the now-live NEW vault so recovery re-verification fails.
        std::fs::write(home_vault(&fx.home), b"corrupted new vault bytes").unwrap();
        recover_if_pending(&fx.home).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes, "original restored");
        assert!(
            !fx.blob("NEWENTRY.blob").exists(),
            "new blob dropped on rollback"
        );
        fx.assert_clean(false);
    }

    // ---- slice 5.2.1: `snapshots/` preservation across a revert swap (§A / Decision ⑮) ----

    /// A `commit_preserving(&["snapshots"])` reverts the vault to NEW **and** carries the live
    /// `snapshots/` store across the swap — the revert never deletes its own safety net.
    #[test]
    fn revert_preserves_snapshots_across_full_commit() {
        let fx = Fixture::new(true, true);
        fx.seed_live_snapshot();
        fx.commit_until_preserving(None, &["snapshots"]).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes, "vault reverted to NEW");
        assert!(
            fx.snapshot_marker_present(),
            "the snapshot store survived the swap"
        );
        fx.assert_clean(true);
    }

    /// 🔴 The snapshots-survival crash matrix: crash at EVERY checkpoint of a preserving swap,
    /// recover, and assert the snapshot store survives in BOTH the roll-forward and roll-back
    /// directions (the vault itself lands NEW for ≥`HomeAside`, OLD for `Staged`).
    #[test]
    fn revert_preserves_snapshots_at_every_checkpoint() {
        use Checkpoint::{
            HomeAsideJournal, HomeMoved, HomeSwapped, OldCleaned, Staged, SubdirsPreserved,
            SwappedJournal,
        };
        let checkpoints = [
            Staged,
            HomeAsideJournal,
            HomeMoved,
            SwappedJournal,
            HomeSwapped,
            SubdirsPreserved,
            OldCleaned,
        ];
        for cp in checkpoints {
            let fx = Fixture::new(true, true);
            fx.seed_live_snapshot();
            let _ = fx.commit_until_preserving(Some(cp), &["snapshots"]);
            recover_if_pending(&fx.home).unwrap();

            let expect_new = cp != Staged;
            if expect_new {
                assert_eq!(
                    fx.live_vault_bytes(),
                    fx.new_bytes,
                    "checkpoint {cp:?}: NEW"
                );
            } else {
                assert_eq!(
                    fx.live_vault_bytes(),
                    fx.old_bytes,
                    "checkpoint {cp:?}: OLD"
                );
            }
            assert!(
                fx.snapshot_marker_present(),
                "checkpoint {cp:?}: the snapshot store must survive"
            );
            fx.assert_clean(expect_new);
            assert!(recover_if_pending(&fx.home).unwrap().is_none());
        }
    }

    /// 🔴 The symmetric-reclaim path: crash AFTER `snapshots/` was moved into the new home but
    /// before cleanup, then corrupt the new vault so recovery re-verification fails and rolls
    /// back. The rollback must swing `snapshots/` back with the original — never delete it.
    #[test]
    fn revert_rollback_keeps_snapshots() {
        let fx = Fixture::new(false, true);
        fx.seed_live_snapshot();
        // Stop right after `op_preserve_subdirs` (snapshots now in the new home, `.old` still
        // present) — the exact window the symmetric reclaim guards.
        let _ = fx.commit_until_preserving(Some(Checkpoint::SubdirsPreserved), &["snapshots"]);
        // Corrupt the now-live NEW vault so re-verification fails → forced rollback.
        std::fs::write(home_vault(&fx.home), b"corrupted new vault bytes").unwrap();

        recover_if_pending(&fx.home).unwrap();
        assert_eq!(fx.live_vault_bytes(), fx.old_bytes, "rolled back to OLD");
        assert!(
            fx.snapshot_marker_present(),
            "🔴 a rollback must keep the snapshot store, not delete it"
        );
        fx.assert_clean(false);
    }

    // ---- slice 5.8: the RETIRING swap (`preserve = &[]`) — the mirror of the revert tests ----

    /// 🔴 A `commit_swap(preserve = &[])` (what `rekey_vault` uses) RETIRES the live `snapshots/`
    /// store — the opposite of a revert (③): a pre-re-key snapshot's bodies are on the OLD DEKs,
    /// so keeping it revertable would defeat the re-key.
    #[test]
    fn retiring_swap_drops_the_live_snapshot_store() {
        let fx = Fixture::new(true, true);
        fx.seed_live_snapshot();
        fx.commit_until(None).unwrap(); // `commit_until` = preserve `&[]`
        assert_eq!(fx.live_vault_bytes(), fx.new_bytes, "vault swapped to NEW");
        assert!(
            !fx.snapshot_marker_present(),
            "an empty-preserve swap must RETIRE the pre-re-key snapshot store"
        );
        fx.assert_clean(true);
    }

    /// 🔴 The retire crash matrix: crash at EVERY checkpoint of a retiring swap. On roll-forward the
    /// snapshot store is GONE (retired with the swapped-in home); on a `Staged` roll-back nothing
    /// happened, so the original store (and its marker) survives.
    #[test]
    fn retiring_swap_drops_snapshots_at_every_checkpoint() {
        use Checkpoint::{
            HomeAsideJournal, HomeMoved, HomeSwapped, OldCleaned, Staged, SubdirsPreserved,
            SwappedJournal,
        };
        let checkpoints = [
            Staged,
            HomeAsideJournal,
            HomeMoved,
            SwappedJournal,
            HomeSwapped,
            SubdirsPreserved,
            OldCleaned,
        ];
        for cp in checkpoints {
            let fx = Fixture::new(true, true);
            fx.seed_live_snapshot();
            let _ = fx.commit_until(Some(cp)); // preserve `&[]`
            recover_if_pending(&fx.home).unwrap();

            let expect_new = cp != Staged;
            if expect_new {
                assert_eq!(
                    fx.live_vault_bytes(),
                    fx.new_bytes,
                    "checkpoint {cp:?}: NEW"
                );
                assert!(
                    !fx.snapshot_marker_present(),
                    "checkpoint {cp:?}: a roll-forward RETIRES the snapshot store"
                );
            } else {
                assert_eq!(
                    fx.live_vault_bytes(),
                    fx.old_bytes,
                    "checkpoint {cp:?}: OLD"
                );
                assert!(
                    fx.snapshot_marker_present(),
                    "checkpoint {cp:?}: a roll-back keeps the original store"
                );
            }
            fx.assert_clean(expect_new);
            assert!(recover_if_pending(&fx.home).unwrap().is_none());
        }
    }
}
