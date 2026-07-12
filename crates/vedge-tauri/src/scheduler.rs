//! In-process periodic maintenance (slice 4.5a; extended 4.5b; generalized to a
//! `ScheduledJob` registry in 4.6a).
//!
//! The Tauri process is the only process — there is no Elixir sidecar (that was
//! aspirational and has been retired). This one background task is the caller the
//! sidecar was imagined to be. It runs on a single 5 s [`TICK`] and drives a list
//! of [`ScheduledJob`]s, each fired on **its own** interval:
//!
//! 1. **[`TtlReapJob`]** (per-tick) — enforces the hard session TTL: past-deadline
//!    sessions are evicted + audit-locked *on a timer*, not only when a command
//!    next touches them (a hung/suspended `WebView` issues no such command). It
//!    also audits handles a TTL-expired `get_session` parked on `pending_locks`
//!    (slice 4.6a, A4 — closing 4.5a's audit-silent eviction).
//! 2. **[`ScreenLockJob`]** (per-tick, Windows only) — locks every open vault on a
//!    `false → true` OS screen-lock edge (slice 4.5b).
//! 3. **[`MaintenanceJob`]** (6 h) — runs `run_maintenance` (trash + audit
//!    retention, orphaned-blob reaping) over every open vault (slice 4.6a).
//!
//! The `ScheduledJob` trait was dropped at 4.5a on a premise that was already
//! false (`async-trait` was thought to be a missing dep — 4.4 had added it). With
//! a third job, per-job intervals, and per-job state all arriving here, it earns
//! its place: state lives *in the job*, not as loop-frame locals.
//!
//! Each due job runs on **its own task** ([`run_tick`] spawns, never awaits inline):
//! `MaintenanceJob` is the first job to lock a *live* session and do slow blob I/O,
//! so a sequential loop would let it stall — or, on a panic, abort — the
//! security-critical reaper + screen-lock. Per-task spawn isolates both.
//!
//! Both lock paths (reap, screen-lock) go through the shared [`lock_handles`]
//! **audit-then-drop**: append `Locked` through the session's own `repo` `Arc`
//! *before* dropping the handle, so the audit trail records the timer-driven lock
//! (this is the shape that retired `lock_vault`'s old `Arc::try_unwrap` × 256 spin).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::task::JoinHandle;

use vedge_core::application::vault::ports::ScreenLockWatcher;
use vedge_core::{record_lock, run_maintenance};

use crate::state::{
    SessionHandle, SessionMap, drain_all_sessions, drain_expired_sessions, drain_pending_locks,
    snapshot_sessions,
};

/// Base tick. Every job is polled on this cadence and fires when its own interval
/// has elapsed. Sub-minute so an expired KEK zeroizes promptly and a screen-lock
/// is noticed within seconds, yet coarse enough to be negligible when idle.
const TICK: Duration = Duration::from_secs(5);

/// Maintenance cadence. Retention windows are measured in **days**, so a 6 h sweep
/// is ample — and it keeps the idle app from doing a blob-directory walk + two
/// `DELETE`s per open vault every 5 s (every run after the first deletes nothing).
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Shared, cheaply-cloned context handed to every job's [`ScheduledJob::run`].
/// Deliberately **not** `AppState`: a job gets exactly the registry handles it
/// needs, nothing more. Grows one field per new job need. `Clone` so each job can
/// take an owned copy onto its own task (see [`run_tick`]).
#[derive(Clone)]
pub(crate) struct SchedulerCtx {
    sessions: Arc<StdMutex<SessionMap>>,
    pending_locks: Arc<StdMutex<Vec<SessionHandle>>>,
}

/// One background job on the shared [`TICK`]. Each due job runs on its **own task**
/// (see [`run_tick`]): a slow job can't stall the others, and a panic is isolated to
/// that task instead of aborting the loop — so the security-critical reaper survives
/// a fault in, say, the heavier maintenance job. `run` also handles its own `Err`s.
#[async_trait]
pub(crate) trait ScheduledJob: Send + Sync {
    /// Stable name, for diagnostics.
    fn name(&self) -> &'static str;
    /// How often this job fires. A job with `interval() <= TICK` fires every tick.
    fn interval(&self) -> Duration;
    /// Do the work. Must handle its own errors — the loop does not.
    async fn run(&self, ctx: &SchedulerCtx);
}

/// Spawn the periodic scheduler onto Tauri's async runtime. Holds only the
/// registry handles (via [`SchedulerCtx`]) and the job list — not the whole
/// [`AppState`](crate::state::AppState). Lives for the process lifetime.
pub(crate) fn spawn(
    sessions: Arc<StdMutex<SessionMap>>,
    pending_locks: Arc<StdMutex<Vec<SessionHandle>>>,
    screen_lock: Arc<dyn ScreenLockWatcher>,
) {
    let ctx = SchedulerCtx {
        sessions,
        pending_locks,
    };
    let mut scheduled: Vec<Scheduled> = vec![
        Scheduled::new(Arc::new(TtlReapJob)),
        Scheduled::new(Arc::new(MaintenanceJob)),
    ];
    // Only sweep for screen-lock where it's implemented (Windows). The stub always
    // reports "unlocked", so off the supported platforms we simply don't register
    // the job. Checked once — platform support doesn't change at runtime.
    if screen_lock.is_supported() {
        scheduled.push(Scheduled::new(Arc::new(ScreenLockJob::new(screen_lock))));
    }
    tauri::async_runtime::spawn(run_loop(ctx, scheduled));
}

/// A registered job plus its own tick accumulator. Keeping `since` next to the job
/// (not a parallel `Vec<Duration>` indexed by position) is the same "state lives
/// with the job" principle the trait reintroduction is built on.
struct Scheduled {
    job: Arc<dyn ScheduledJob>,
    since: Duration,
}

impl Scheduled {
    fn new(job: Arc<dyn ScheduledJob>) -> Self {
        Self {
            job,
            since: Duration::ZERO,
        }
    }
}

/// The scheduler loop: on each [`TICK`], spawn every job whose interval has elapsed
/// since it last ran. `since` accumulates nominal ticks, so the cadence is
/// independent of any wall clock (and unit-testable via [`tick_due`]).
async fn run_loop(ctx: SchedulerCtx, mut scheduled: Vec<Scheduled>) {
    loop {
        tokio::time::sleep(TICK).await;
        // Fire-and-forget: dropping the handles detaches the job tasks. A slow job
        // (maintenance's blob walk + per-vault lock) then can't stall the loop that
        // drives the security-critical reaper + screen-lock, and a panicking job is
        // isolated to its own task instead of aborting the whole scheduler.
        drop(run_tick(&mut scheduled, &ctx));
    }
}

/// One tick: advance each job's clock and spawn the ones now due on their **own
/// tasks**, returning the handles. Spawning (not awaiting inline) is what keeps a
/// slow/panicking job from blocking or killing the loop; tests await the returned
/// handles for deterministic completion.
fn run_tick(scheduled: &mut [Scheduled], ctx: &SchedulerCtx) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::new();
    for s in scheduled {
        if tick_due(&mut s.since, s.job.interval()) {
            let job = Arc::clone(&s.job);
            let ctx = ctx.clone();
            tracing::trace!(job = job.name(), "scheduler: spawning due job");
            handles.push(tokio::task::spawn(async move { job.run(&ctx).await }));
        }
    }
    handles
}

/// Advance an accumulator by one [`TICK`]; return `true` (and reset it) when the
/// interval is reached. A job with `interval <= TICK` fires every tick.
fn tick_due(since: &mut Duration, interval: Duration) -> bool {
    *since = since.saturating_add(TICK);
    if *since >= interval {
        *since = Duration::ZERO;
        true
    } else {
        false
    }
}

/// Audit-then-drop each handle: append the `Locked` row through the session's own
/// `repo` `Arc` *before* dropping, so the audit trail records the lock and the KEK
/// zeroizes once the last in-flight clone releases. Shared by every lock path.
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

/// Screen-lock poll (slice 4.5b). On a **false → true** edge — the workstation just
/// locked — lock every open vault via the shared audit-then-drop path.
///
/// **Edge, not level:** a screen that stays locked must lock the vault exactly
/// once, not every tick (which would fight a re-unlock while still locked).
/// **Fails open:** a query error logs and does **not** lock (nor disturb the edge
/// state) — a false positive would eject a working user; a false negative only
/// costs a few seconds behind an already-locked screen.
async fn poll_screen_lock(
    sessions: &StdMutex<SessionMap>,
    watcher: &dyn ScreenLockWatcher,
    prev_locked: &AtomicBool,
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
    if locked && !prev_locked.load(Ordering::Relaxed) {
        lock_handles(drain_all_sessions(sessions)).await;
    }
    prev_locked.store(locked, Ordering::Relaxed);
}

// ---- jobs -----------------------------------------------------------------

/// Enforce the hard session TTL: evict + audit-lock past-deadline sessions, and
/// audit the handles a TTL-expired `get_session` parked on `pending_locks` (slice
/// 4.6a, A4). Both are timer-driven TTL locks, so they share this one job.
struct TtlReapJob;

#[async_trait]
impl ScheduledJob for TtlReapJob {
    fn name(&self) -> &'static str {
        "ttl-reap"
    }
    fn interval(&self) -> Duration {
        TICK
    }
    async fn run(&self, ctx: &SchedulerCtx) {
        let mut handles = drain_expired_sessions(&ctx.sessions);
        handles.extend(drain_pending_locks(&ctx.pending_locks));
        lock_handles(handles).await;
    }
}

/// Run `run_maintenance` (trash + audit retention, orphaned-blob reaping) over
/// every open vault. Uses [`snapshot_sessions`] — NON-destructive — so cleaning a
/// vault never locks it (the pinned `maintenance_does_not_lock_vaults`).
struct MaintenanceJob;

#[async_trait]
impl ScheduledJob for MaintenanceJob {
    fn name(&self) -> &'static str {
        "maintenance"
    }
    fn interval(&self) -> Duration {
        MAINTENANCE_INTERVAL
    }
    async fn run(&self, ctx: &SchedulerCtx) {
        for (_, handle) in snapshot_sessions(&ctx.sessions) {
            let guard = handle.lock().await;
            match run_maintenance(&guard).await {
                Ok(report) => {
                    if report.trashed_entries_deleted > 0
                        || report.audit_events_deleted > 0
                        || report.orphaned_blobs_deleted > 0
                    {
                        tracing::info!(
                            trashed = report.trashed_entries_deleted,
                            audit = report.audit_events_deleted,
                            blobs = report.orphaned_blobs_deleted,
                            "scheduler: maintenance purged expired data"
                        );
                    }
                }
                Err(e) => tracing::warn!(error = %e, "scheduler: maintenance run failed"),
            }
            drop(guard);
        }
    }
}

/// Poll the OS screen-lock state and lock all vaults on a lock edge (slice 4.5b).
/// Owns its own edge state (`prev_locked`) — absorbed from 4.5b's loop-frame local.
struct ScreenLockJob {
    watcher: Arc<dyn ScreenLockWatcher>,
    prev_locked: AtomicBool,
}

impl ScreenLockJob {
    fn new(watcher: Arc<dyn ScreenLockWatcher>) -> Self {
        Self {
            watcher,
            prev_locked: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl ScheduledJob for ScreenLockJob {
    fn name(&self) -> &'static str {
        "screen-lock"
    }
    fn interval(&self) -> Duration {
        TICK
    }
    async fn run(&self, ctx: &SchedulerCtx) {
        poll_screen_lock(&ctx.sessions, self.watcher.as_ref(), &self.prev_locked).await;
    }
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

    use std::sync::atomic::AtomicUsize;

    use super::*;
    use crate::error::CommandError;
    use crate::test_support::{state_fixture, unlocked_vault};
    use vedge_core::domain::vault::entities::{AuditAction, AuditQuery};
    use vedge_core::domain::vault::errors::VaultError;
    use vedge_core::infrastructure::screen_lock::MemoryScreenLockWatcher;

    fn ctx_of(state: &crate::state::AppState) -> SchedulerCtx {
        SchedulerCtx {
            sessions: state.sessions_handle(),
            pending_locks: state.pending_locks_handle(),
        }
    }

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

    /// A job that just counts its runs, with a configurable interval.
    struct CountingJob {
        interval: Duration,
        count: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl ScheduledJob for CountingJob {
        fn name(&self) -> &'static str {
            "counting"
        }
        fn interval(&self) -> Duration {
            self.interval
        }
        async fn run(&self, _ctx: &SchedulerCtx) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[tokio::test]
    async fn screen_lock_edge_drives_lock_and_audits() {
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        let sessions = state.sessions_handle();

        // Hold a clone so the session survives the sweep's drop and we can query
        // its audit log afterwards.
        let held = state.get_session(&vault_id).unwrap();

        let watcher = MemoryScreenLockWatcher::new();
        watcher.set_locked(true);
        let prev = AtomicBool::new(false);
        poll_screen_lock(&sessions, &watcher, &prev).await;

        assert!(prev.load(Ordering::Relaxed), "the edge should be recorded");
        assert!(
            !state.is_unlocked(&vault_id),
            "a screen-lock edge locks the open vault"
        );

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
        // (not an edge) must NOT lock the still-open vault.
        let prev = AtomicBool::new(true);
        poll_screen_lock(&sessions, &watcher, &prev).await;
        assert!(
            state.is_unlocked(&vault_id),
            "a persistently-locked screen must not re-lock the vault"
        );

        // Drive a real edge: unlock (false) then lock (true) → false→true.
        watcher.set_locked(false);
        poll_screen_lock(&sessions, &watcher, &prev).await;
        assert!(
            !prev.load(Ordering::Relaxed),
            "the unlock should clear the edge state"
        );
        watcher.set_locked(true);
        poll_screen_lock(&sessions, &watcher, &prev).await;
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

        let prev = AtomicBool::new(false);
        poll_screen_lock(&sessions, &FailingWatcher, &prev).await;
        assert!(
            state.is_unlocked(&vault_id),
            "a query error must NOT lock the vault (fail open)"
        );
        assert!(
            !prev.load(Ordering::Relaxed),
            "a query error must not disturb the edge state"
        );
    }

    #[tokio::test]
    async fn maintenance_does_not_lock_vaults() {
        // The snapshot-vs-drain trap: running maintenance must clean a vault
        // WITHOUT locking it.
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        let ctx = ctx_of(&state);

        MaintenanceJob.run(&ctx).await;

        assert!(
            state.is_unlocked(&vault_id),
            "maintenance must not lock any open vault"
        );
    }

    #[tokio::test]
    async fn expired_session_eviction_is_audited() {
        // A4: a TTL-expired `get_session` evicts audit-silently inline (it's sync)
        // but parks the handle on `pending_locks`; the reaper then audits it.
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;

        // Hold a clone so we can read the audit log after the reaper drops its own.
        let held = state.get_session(&vault_id).unwrap();

        state.touch_deadline(&vault_id, Some(Duration::ZERO));
        match state.get_session(&vault_id) {
            Err(CommandError::SessionExpired) => {}
            other => panic!("expected SessionExpired, got {other:?}"),
        }
        assert!(!state.is_unlocked(&vault_id), "the slot was evicted");

        // The reaper drains the pending queue and lands the Locked row.
        TtlReapJob.run(&ctx_of(&state)).await;

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
        assert!(
            page.total >= 1,
            "a TTL-expired get_session eviction must eventually be audited"
        );
    }

    #[tokio::test]
    async fn ttl_reap_job_locks_and_audits_expired_session() {
        // The reaper's PRIMARY path: an idle session that times out WITHOUT ever
        // being touched by `get_session` (the `drain_expired_sessions` half of
        // TtlReapJob, distinct from the `pending_locks` half above).
        let dir = tempfile::tempdir().unwrap();
        let (state, vault_id) = unlocked_vault(&dir, "v.vdb").await;
        // Hold a clone so we can read the audit log after the reaper drops its own.
        let held = state.get_session(&vault_id).unwrap();

        state.touch_deadline(&vault_id, Some(Duration::ZERO)); // expire in place
        TtlReapJob.run(&ctx_of(&state)).await;

        assert!(
            !state.is_unlocked(&vault_id),
            "the reaper drains the expired slot"
        );
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
        assert!(page.total >= 1, "the reaper must audit a Locked row");
    }

    #[test]
    fn maintenance_job_respects_its_interval() {
        // A 6 h job does not fire within many 5 s ticks...
        let mut since = Duration::ZERO;
        for _ in 0..20 {
            assert!(!tick_due(&mut since, MaintenanceJob.interval()));
        }
        // ...while a per-tick job fires on every tick.
        let mut s = Duration::ZERO;
        assert!(tick_due(&mut s, TtlReapJob.interval()));
        assert!(tick_due(&mut s, TtlReapJob.interval()));
    }

    #[tokio::test]
    async fn scheduled_jobs_run_independently() {
        // Two jobs on one loop: the fast one fires every tick, the slow one not at
        // all within the window — each on its own schedule.
        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        let ctx = ctx_of(&state);

        let fast = Arc::new(AtomicUsize::new(0));
        let slow = Arc::new(AtomicUsize::new(0));
        let mut scheduled = vec![
            Scheduled::new(Arc::new(CountingJob {
                interval: TICK,
                count: Arc::clone(&fast),
            })),
            Scheduled::new(Arc::new(CountingJob {
                interval: MAINTENANCE_INTERVAL,
                count: Arc::clone(&slow),
            })),
        ];
        for _ in 0..3 {
            // Await the spawned handles so the assertions see completed runs.
            for h in run_tick(&mut scheduled, &ctx) {
                h.await.unwrap();
            }
        }

        assert_eq!(
            fast.load(Ordering::Relaxed),
            3,
            "a per-tick job fires every tick"
        );
        assert_eq!(
            slow.load(Ordering::Relaxed),
            0,
            "a 6 h job does not fire within 3 ticks"
        );
    }
}
