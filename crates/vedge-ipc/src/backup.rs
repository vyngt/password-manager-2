//! Backup wire DTOs (slices 5.2 / 5.2.2).
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
    /// Whole-archive BLAKE3 (lowercase hex); also written to the `.blake3` sidecar, and — as
    /// of 5.2.2 (finding L2) — actually verified against it when the archive is next read.
    pub archive_blake3: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis-`Z`.
    pub created_at: String,
}

/// What is actually at a Replace target.
///
/// 🔴 `Missing` and `Unreadable` are **not** the same answer, and collapsing them into one
/// "no identity" is finding **H2**: it let an unreadable target silently disarm the
/// uuid-mismatch guard. The UI must be able to tell them apart, because one means *"you want
/// Open backup"* and the other means *"say out loud that you accept replacing a vault we could
/// not identify."*
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetStateDto {
    Missing,
    Unreadable,
    Readable,
}

/// A read-only preview of what opening or replacing with a `.vbk` would do: the backup's own
/// facts, the target's facts (Replace), the destination's (Open), and every computed hard-stop.
// The bools are independent refusal reasons, not a single collapsible state.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPreviewDto {
    pub format_version: u32,
    pub backup_vault_uuid: Option<String>,
    pub backup_schema_version: i32,
    pub created_at: String,
    pub backup_entry_count: u64,
    pub backup_blob_count: u64,

    // ---- the current target (Replace mode) ----
    pub target_state: TargetStateDto,
    pub target_uuid: Option<String>,
    /// 🔴 `Option`, never a bare count. **Never render `0` for an unknown entry count** — a
    /// zero the user believes is a zero the user acts on.
    pub target_entry_count: Option<u64>,
    pub target_last_unlocked_at: Option<String>,
    pub target_commit_counter: Option<i64>,

    // ---- hard stops ----
    pub uuid_mismatch: bool,
    pub unknown_format: bool,
    pub unknown_schema: bool,
    /// Open mode: something already exists at the destination. No override exists — the UI must
    /// block, not offer a "do it anyway".
    pub dest_occupied: bool,

    // ---- risks, each needing its OWN acknowledgement ----
    pub rollback_delta: Option<i64>,
    /// **M3.** `Some(true)` ⇒ this backup needs the master password / Secret Key in force on
    /// `created_at`, including the Emergency Kit printed then.
    ///
    /// 🔴 `None` ⇒ **unknown**, and must be *shown* as unknown. It is never `Some(false)` unless
    /// both sides were read and genuinely matched.
    pub credentials_differ: Option<bool>,
    /// **②.** A live vault on this machine already carries this backup's identity, so opening it
    /// makes a COPY — which will be given a fresh identity of its own.
    pub duplicate_of: Option<String>,
}

/// The result of a successful `open_backup` (slice 5.2.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBackupReportDto {
    pub home: String,
    pub vault_uuid: Option<String>,
    /// ②: the live vault this was recognised as a copy of. `Some` ⇔ `fresh_uuid`.
    pub duplicate_of: Option<String>,
    pub fresh_uuid: bool,
    /// The Secret Key was carried across to the new identity, so the copy opens with the master
    /// password alone. `false` ⇒ the Emergency Kit is needed — correct, not an error.
    pub secret_key_copied: bool,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis-`Z`.
    pub opened_at: String,
}

/// The result of a successful `replace_vault_from_backup`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceReportDto {
    pub vault_uuid: Option<String>,
    pub entry_count: u64,
    pub blob_count: u64,
    /// RFC-3339 millis-`Z`.
    pub restored_at: String,
    /// ⑭: the `pre-restore` snapshot holding the state this replace overwrote — the undo point.
    pub undo_snapshot_id: Option<String>,
}

/// The result of a successful `delete_vault` (slice 5.2.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteVaultReportDto {
    /// Snapshots destroyed with the home (for the confirmation copy).
    pub snapshots_deleted: u64,
    /// Approximate bytes reclaimed.
    pub bytes_freed: u64,
    /// 🔴 `false` ⇒ the uuid was unknowable or a credential delete failed. The UI MUST say so
    /// and point at `mise keychain-audit` — silence recreates the orphan this slice prevents.
    pub credentials_cleaned: bool,
}

/// Read-only stats for the vault-details dialog (slice 5.2.4).
///
/// All best-effort — `0` on any read error, never a failure. The copyable Vault ID rides on
/// `RegisteredVaultStatusDto`, so it is not repeated here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDetailsDto {
    pub entry_count: u64,
    pub snapshot_count: u64,
    pub on_disk_bytes: u64,
}

/// The vault's backup health (Decision ⑧).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatusDto {
    /// 🔴 When a `.vbk` was last written — and **only** that. A snapshot is not a backup: it
    /// lives on the same disk, in the same folder, and dies with it. Reading this from
    /// `BackupCreated` audit rows would let a snapshot silence "you have never backed up",
    /// which is exactly the reassurance a user must never be given falsely.
    pub last_backup_at: Option<String>,
    /// When a local snapshot was last taken. Shown beside, never instead of, the above.
    pub last_snapshot_at: Option<String>,
}
