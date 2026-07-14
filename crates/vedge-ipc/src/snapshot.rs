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
