//! In-process periodic maintenance (slice 4.5a).
//!
//! The Tauri process is the only process — there is no Elixir sidecar (that was
//! aspirational and has been retired). This background task is the caller the
//! sidecar was imagined to be. Today it runs one job — the **session reaper** —
//! which enforces the hard session TTL: sessions whose absolute deadline has
//! passed are evicted and their KEK zeroized *on a timer*, not only when a
//! command next touches them. That distinction is the whole point: a hung,
//! detached, or suspended `WebView` never issues that next command, so without the
//! reaper the KEK would stay hot indefinitely. Slice 4.5b bolts a second job (OS
//! screen-lock) onto the same tick.
//!
//! Reaping is **audit-then-drop**: append the `Locked` row through the session's
//! own `repo` `Arc` *before* dropping the handle, so the audit trail (slice 4.1)
//! records timer-driven locks. This is the shape that retired `lock_vault`'s old
//! `Arc::try_unwrap` × 256 spin, which dropped the row under contention. (The
//! reaper is the *normal* audit path, not the sole one: a command that touches an
//! already-expired session evicts it audit-silently via `get_session` first — a
//! rare race, since an idle vault issues no such command before this tick.)

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use vedge_core::record_lock;

use crate::state::{SessionMap, drain_expired_sessions};

/// How often the scheduler sweeps. Sub-minute so an expired session's KEK is
/// zeroized promptly, yet coarse enough to be negligible when the app is idle.
const TICK: Duration = Duration::from_secs(5);

/// Spawn the periodic scheduler onto Tauri's async runtime.
///
/// Holds only a clone of the session-registry handle — not the whole
/// [`AppState`](crate::state::AppState) — so the background task stays decoupled
/// from command wiring. The task lives for the process lifetime.
pub(crate) fn spawn(sessions: Arc<StdMutex<SessionMap>>) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            reap_expired(&sessions).await;
        }
    });
}

/// One sweep: evict every past-deadline session and land its `Locked` audit row,
/// then drop the handle so `VaultSession::Drop` zeroizes the KEK + index once the
/// last in-flight clone releases.
async fn reap_expired(sessions: &StdMutex<SessionMap>) {
    // Drain under the std mutex (never held across an await), then audit each.
    for handle in drain_expired_sessions(sessions) {
        let guard = handle.lock().await;
        if let Err(e) = record_lock(&guard).await {
            tracing::warn!(error = %e, "session reaper: failed to append Locked audit row");
        }
        drop(guard);
        drop(handle);
    }
}
