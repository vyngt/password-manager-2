use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::application::vault::ports::screen_lock::ScreenLockWatcher;
use crate::domain::vault::errors::VaultError;

/// In-memory screen-lock watcher for tests and e2e.
///
/// Reports supported and never touches the OS, so the full lock-on-screen-lock
/// path (scheduler edge detection → session reap → frontend WASM wipe) can be
/// exercised headlessly — `WebDriver` cannot lock a real workstation. Not for
/// production.
///
/// Two levers, for two callers:
/// - **Unit tests** flip the state directly with [`set_locked`](Self::set_locked)
///   — the settable `AtomicBool`, for precise edge / level / fail-open coverage.
/// - **The e2e** runs in a *separate process*, so it uses a **sentinel file**:
///   [`watching_file`](Self::watching_file) reports *locked* exactly while that
///   file exists on disk. The harness does all its setup, then creates the file
///   to trigger a **deterministic** `false → true` edge on a live, unlocked
///   session — no timing race (unlike a wall-clock delay).
pub struct MemoryScreenLockWatcher {
    /// Explicit lock state (unit tests). OR-ed with `sentinel` below.
    locked: AtomicBool,
    /// Optional sentinel file: reported *locked* while it exists on disk (the
    /// e2e's deterministic lever). `None` for the settable-only unit double.
    sentinel: Option<PathBuf>,
}

impl MemoryScreenLockWatcher {
    /// A settable-only watcher that starts unlocked (unit tests).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
            sentinel: None,
        }
    }

    /// A watcher that reports *locked* while `sentinel` exists on disk (the e2e's
    /// lever). Starts unlocked (the file doesn't exist yet), so a vault can be
    /// opened first; creating the file then produces the `false → true` edge the
    /// scheduler locks on.
    #[must_use]
    pub const fn watching_file(sentinel: PathBuf) -> Self {
        Self {
            locked: AtomicBool::new(false),
            sentinel: Some(sentinel),
        }
    }

    /// Flip the simulated screen-lock state (unit tests).
    pub fn set_locked(&self, locked: bool) {
        self.locked.store(locked, Ordering::SeqCst);
    }
}

impl Default for MemoryScreenLockWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenLockWatcher for MemoryScreenLockWatcher {
    fn is_supported(&self) -> bool {
        true
    }

    fn is_screen_locked(&self) -> Result<bool, VaultError> {
        if self.locked.load(Ordering::SeqCst) {
            return Ok(true);
        }
        Ok(self.sentinel.as_ref().is_some_and(|p| p.exists()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn memory_watcher_is_supported_and_settable() {
        let w = MemoryScreenLockWatcher::new();
        assert!(w.is_supported());
        assert!(matches!(w.is_screen_locked(), Ok(false)));
        w.set_locked(true);
        assert!(matches!(w.is_screen_locked(), Ok(true)));
        w.set_locked(false);
        assert!(matches!(w.is_screen_locked(), Ok(false)));
    }

    #[test]
    fn sentinel_file_drives_lock_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sentinel = dir.path().join("screen-locked.flag");
        let w = MemoryScreenLockWatcher::watching_file(sentinel.clone());
        // Absent → unlocked.
        assert!(matches!(w.is_screen_locked(), Ok(false)));
        // Present → locked.
        std::fs::write(&sentinel, b"1").expect("touch sentinel");
        assert!(matches!(w.is_screen_locked(), Ok(true)));
        // Removed → unlocked again (models the screen being unlocked).
        std::fs::remove_file(&sentinel).expect("rm sentinel");
        assert!(matches!(w.is_screen_locked(), Ok(false)));
    }
}
