//! Session-only holder for the last password-health scan report (slice 4.3).
//!
//! The report carries **no secrets** — only non-invertible derivatives (see
//! `vedge_ipc::health`) — but it *is* sensitive derived data: it names which of
//! your entries are weak, reused, or old. So it lives only for the session: held
//! in an app-root [`HealthReportCtx`] and emptied whenever the vault locks (the
//! prev-guarded `Effect` in `app.rs`, the same choke point that clears the
//! generator history). Ephemeral by design — never persisted; re-derived on
//! demand by re-running the scan.

use leptos::prelude::*;
use vedge_ipc::HealthReportDto;

/// App-root reactive context holding the most recent scan report, or `None`
/// before the first scan and after a lock. `RwSignal<T>` is `Copy` regardless of
/// `T`, so the wrapper is `Copy` (mirrors `GeneratedHistoryCtx`).
#[derive(Clone, Copy)]
pub struct HealthReportCtx {
    /// The last scan's report; `None` until a scan runs, and after wipe-on-lock.
    pub report: RwSignal<Option<HealthReportDto>>,
}

impl HealthReportCtx {
    /// Construct an empty context (no scan yet).
    #[must_use]
    pub fn new() -> Self {
        Self {
            report: RwSignal::new(None),
        }
    }

    /// Replace the held report with a fresh scan result.
    pub fn set(&self, report: HealthReportDto) {
        self.report.set(Some(report));
    }

    /// Drop the held report (wipe-on-lock); mirrors `GeneratedHistoryCtx::clear`.
    pub fn clear(&self) {
        self.report.set(None);
    }
}

impl Default for HealthReportCtx {
    fn default() -> Self {
        Self::new()
    }
}
