//! Backup wire DTOs (slice 5.2).
//!
//! Derivatives only — counts, hashes, a timestamp, and a path. No payload and no
//! key material ever crosses this boundary (Decision ④: the archive is
//! ciphertext, but its *contents* never round-trip through a DTO).

use serde::{Deserialize, Serialize};

/// The result of a successful `backup_vault`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupReportDto {
    /// Absolute path of the written `.vbk` archive.
    pub archive_path: String,
    pub archive_bytes: u64,
    /// Whole-archive BLAKE3 (lowercase hex); also written to the `.blake3` sidecar.
    pub archive_blake3: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis-`Z`.
    pub created_at: String,
}

/// A read-only preview of what restoring a `.vbk` would do (slice 5.2b): the
/// backup's own facts, the current target's facts, and the computed hard-stops.
// The four bools are independent refusal reasons, not a single collapsible state.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPreviewDto {
    pub format_version: u32,
    pub backup_vault_uuid: Option<String>,
    pub backup_schema_version: i32,
    pub created_at: String,
    pub backup_entry_count: u64,
    pub backup_blob_count: u64,
    pub target_uuid: Option<String>,
    pub target_entry_count: Option<u64>,
    pub target_last_unlocked_at: Option<String>,
    /// `None` until slice 5.2c wires the `commit_counter` column.
    pub target_commit_counter: Option<i64>,
    pub uuid_mismatch: bool,
    pub unknown_format: bool,
    pub unknown_schema: bool,
    pub target_unreadable: bool,
    /// The rollback framing; `None` until 5.2c.
    pub rollback_delta: Option<i64>,
}

/// The result of a successful `restore_vault`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreReportDto {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis-`Z`.
    pub restored_at: String,
}
