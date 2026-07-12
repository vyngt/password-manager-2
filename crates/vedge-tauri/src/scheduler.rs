//! In-process periodic maintenance (slice 4.5a; extended in 4.5b).
//!
//! The Tauri process is the only process — there is no Elixir sidecar (that was
//! aspirational and has been retired). This background task is the caller the
//! sidecar was imagined to be. It runs on one 5 s tick and does two jobs:
//!
//! 1. **Session reaper** (4.5a) — enforces the hard session TTL: sessions whose
//!    absolute deadline has passed are evicted and their KEK zeroized *on a
//!    timer*, not only when a command next touches them. That distinction is the
//!    whole point: a hung, detached, or suspended `WebView` never issues that
//!    next command, so without the reaper the KEK would stay hot indefinitely.
//! 2. **Screen-lock sweep** (4.5b) — on a `false → true` screen-lock edge (the
//!    workstation just locked) locks every open vault. Same 5 s tick; edge-, not
//!    level-triggered, so a screen that stays locked doesn't re-lock every tick.
//!
//! Both jobs lock **audit-then-drop**: append the `Locked` row through the
//! session's own `repo` `Arc` *before* dropping the handle, so the audit trail
//! (slice 4.1) records the timer-driven lock. This is the shape that retired
//! `lock_vault`'s old `Arc::try_unwrap` × 256 spin, which dropped the row under
//! contention. (The reaper is the *normal* audit path for a TTL lock, not the
//! sole one: a command that touches an already-expired session evicts it
//! audit-silently via `get_session` first — a rare race, since an idle vault
//! issues no such command before this tick.)

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use vedge_core::application::vault::ports::ScreenLockWatcher;
use vedge_core::record_lock;

use crate::state::{SessionHandle, SessionMap, drain_all_sessions, drain_expired_sessions};

/// How often the scheduler sweeps. Sub-minute so an expired session's KEK is
/// zeroized promptly (and a screen-lock is noticed within a few seconds), yet
/// coarse enough to be negligible when the app is idle.
const TICK: Duration = Duration::from_secs(5);

/// Spawn the periodic scheduler onto Tauri's async runtime.
///
/// Holds only a clone of the session-registry handle and the screen-lock watcher
/// — not the whole [`AppState`](crate::state::AppState) — so the background task
/// stays decoupled from command wiring. The task lives for the process lifetime.
pub(crate) fn spawn(sessions: Arc<StdMutex<SessionMap>>, screen_lock: Arc<dyn ScreenLockWatcher>) {
    tauri::async_runtime::spawn(async move {
        // Only sweep for screen-lock where it's implemented (Windows). Off the
        // supported platforms the stub always reports "unlocked" anyway, so skip the
        // per-tick call. Checked once — platform support doesn't change at runtime.
        let screen_lock_supported = screen_lock.is_supported();
        // Edge state for the screen-lock sweep (see [`poll_screen_lock`]).
        let mut screen_was_locked = false;
        loop {
            tokio::time::sleep(TICK).await;
            reap_expired(&sessions).await;
            if screen_lock_supported {
                poll_screen_lock(&sessions, screen_lock.as_ref(), &mut screen_was_locked).await;
            }
        }
    });
}

/// Audit-then-drop each handle: append the `Locked` row through the session's own
/// `repo` `Arc` *before* dropping, so the audit trail records the lock and the KEK
/// zeroizes once the last in-flight clone releases. Shared by both jobs.
async fn lock_handles(handles: Vec<SessionHandle>) {
    for handle in handles {
        let guard = handle.lock().await;
        if let Err(e) = record_lock(&guard).await {
            tracing::warn!(error = %e, "scheduler: failed to append Locked audit row");
        }
        drop(guard);
        drop(handle);
    }
}

/// One reaper sweep: evict every past-deadline session and land its `Locked`
/// audit row, then drop so `VaultSession::Drop` zeroizes the KEK + index once the
/// last in-flight clone releases.
async fn reap_expired(sessions: &StdMutex<SessionMap>) {
    lock_handles(drain_expired_sessions(sessions)).await;
}

/// One screen-lock sweep (slice 4.5b). On a **false → true** edge — the
/// workstation just locked — lock every open vault via the same audit-then-drop
/// path; the frontend `is_unlocked` poll then drives the WASM-plaintext wipe.
///
/// **Edge, not level:** a screen that stays locked must lock the vault exactly
/// once, not every tick (which would fight a re-unlock while still locked).
///
/// **Fails open:** a query error logs and does **not** lock (nor does it disturb
/// the edge state). A false positive would eject a working user mid-task; a false
/// negative only costs a few seconds behind an already-locked screen.
async fn poll_screen_lock(
    sessions: &StdMutex<SessionMap>,
    watcher: &dyn ScreenLockWatcher,
    prev_locked: &mut bool,
) {
    let locked = match watcher.is_screen_locked() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "scheduler: screen-lock query failed; failing open (not locking)"
            );
            return;
        }
    };
    if locked && !*prev_locked {
        lock_handles(drain_all_sessions(sessions)).await;
    }
    *prev_locked = locked;
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )]

    use super::*;
    use crate::test_support::unlocked_vault;
    use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};
    use vedge_core::domain::vault::errors::VaultError;
    use vedge_core::infrastructure::screen_lock::MemoryScreenLockWatcher;

    /// A watcher whose query always fails — for the fail-open assertion.
    struct FailingWatcher;
    impl ScreenLockWatcher for FailingWatcher {
        fn is_supported(&self) -> bool {
            true
        }
        fn is_screen_locked(&self) -> Result<bool, VaultError> {
            Err(VaultError::ScreenLockUnavailable)
        }
    }

    #[tokio::test]
    async fn memory_watcher_drives_lock() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        let sessions = state.sessions_handle();

        // Hold a clone of the handle so the session survives the sweep's drop and
        // we can query its audit log afterwards (mirrors the 4.5a reaper test).
        let held = state.get_session(&vault_id).unwrap();

        let watcher = MemoryScreenLockWatcher::new();
        watcher.set_locked(true);
        let mut prev = false;
        poll_screen_lock(&sessions, &watcher, &mut prev).await;

        assert!(prev, "the edge should be recorded");
        assert!(
            !state.is_unlocked(&vault_id),
            "a screen-lock edge locks the open vault"
        );

        // The Locked audit row landed (audit-then-drop). Scope the guard so it
        // drops before the assertion (clippy::significant_drop_tightening).
        let page = {
            let guard = held.lock().await;
            vedge_core::list_audit(
                &guard,
                AuditQuery {
                    actions: vec![AuditAction::Locked],
                    ..Default::default()
                },
            )
            .await
            .unwrap()
        };
        assert!(page.total >= 1, "the screen-lock must audit a Locked row");
    }

    #[tokio::test]
    async fn locks_on_edge_not_level() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        let sessions = state.sessions_handle();
        let watcher = MemoryScreenLockWatcher::new();
        watcher.set_locked(true);

        // `prev == true`: the screen was already locked on a prior tick. A *level*
        // (not an edge) must NOT lock the still-open vault — otherwise a locked
        // screen would fight every re-unlock.
        let mut prev = true;
        poll_screen_lock(&sessions, &watcher, &mut prev).await;
        assert!(
            state.is_unlocked(&vault_id),
            "a persistently-locked screen must not re-lock the vault"
        );

        // Now drive a real edge: unlock (false) then lock (true) → false→true.
        watcher.set_locked(false);
        poll_screen_lock(&sessions, &watcher, &mut prev).await;
        assert!(!prev, "the unlock should clear the edge state");
        watcher.set_locked(true);
        poll_screen_lock(&sessions, &watcher, &mut prev).await;
        assert!(
            !state.is_unlocked(&vault_id),
            "a fresh false→true edge locks the vault"
        );
    }

    #[tokio::test]
    async fn query_failure_fails_open() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        let sessions = state.sessions_handle();

        let mut prev = false;
        poll_screen_lock(&sessions, &FailingWatcher, &mut prev).await;
        assert!(
            state.is_unlocked(&vault_id),
            "a query error must NOT lock the vault (fail open)"
        );
        assert!(!prev, "a query error must not disturb the edge state");
    }
}
