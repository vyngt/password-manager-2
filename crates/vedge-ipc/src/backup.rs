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
