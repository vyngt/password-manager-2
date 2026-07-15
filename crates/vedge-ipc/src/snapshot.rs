//! Snapshot wire DTOs (slice 5.2.1). Plain serde; timestamps are pre-formatted strings.

use serde::{Deserialize, Serialize};

/// One snapshot in the list UI — all plaintext, non-invertible facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDto {
    /// The snapshot's directory name — its stable id.
    pub id: String,
    /// RFC-3339 millis UTC.
    pub created_at: String,
    /// `"manual"` or `"pre-restore"` (Decision ⑭ — the undo point).
    pub reason: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// The snapshot's credentials differ from the live vault (a failed rewrap — it opens only
    /// with the OLD password). Drives the ⚠ stale badge. `false` when the live vault is
    /// unreadable (unknown ⇒ don't alarm), or when the credentials match.
    pub stale_credential: bool,
}

/// The result of "Snapshot now".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotReportDto {
    pub id: String,
    pub created_at: String,
    pub reason: String,
    pub entry_count: u64,
    pub blob_count: u64,
}

/// The result of an in-place revert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertReportDto {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    pub reverted_at: String,
}

/// The result of a SEAMLESS in-place revert from `/v/snapshots` (slice 5.2.3, Decision ⑰).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamlessRevertDto {
    /// 🔴 `true` ⇒ the vault re-opened and the user STAYS on the snapshots page. `false` ⇒ the
    /// swap committed but the vault could not be re-opened (a stale snapshot whose rewrap failed,
    /// **or** a rare post-teardown swap failure) — the UI navigates to the launch screen. Either
    /// way the revert is not a failure to report as one.
    pub stayed_unlocked: bool,
    pub entry_count: u64,
    pub reverted_at: String,
    /// A rollback was detected on the re-opened session (the reverted state is older than this
    /// device's baseline). `None` when the vault did not stay unlocked.
    pub rollback_detected: bool,
}
