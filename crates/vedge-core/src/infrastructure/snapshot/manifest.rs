//! The snapshot manifest — the on-disk descriptor of one local snapshot (slice 5.2.1).
//!
//! Distinct from [`crate::infrastructure::backup::manifest::BackupManifest`], which is a
//! `.vbk` tar's per-member integrity gate. A snapshot is a *directory* inside the vault
//! home's `snapshots/` store, and needs different fields: `reason` (⑭ — tags the
//! auto-snapshot taken before a revert), `verify_hash_prefix` (⑬ — drives the "opens with
//! your current password" badge and confirms a rewrap landed), and the content-addressed
//! **object map** (`objects[]`) the revert path reconstructs `blobs/{entry_id}.blob` from.
//!
//! `manifest.json` is written **last** inside the snapshot's staging dir, so once the dir
//! is atomically renamed into place it is guaranteed complete. It never serializes an
//! `EntryPayload` and never touches the `A3-` Secret Key.

use serde::{Deserialize, Serialize};

/// Bumped ONLY on a breaking change to the snapshot manifest shape.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// The manifest filename inside a snapshot directory (written last = the commit marker).
pub const SNAPSHOT_MANIFEST_NAME: &str = "manifest.json";
/// The snapshot's `.vdb` filename inside a snapshot directory.
pub const SNAPSHOT_VAULT_FILE: &str = "vault.vdb";
/// The content-addressed object pool directory inside the snapshots store.
pub const OBJECTS_DIR: &str = "objects";
/// Filename prefix on a pooled object: `objects/b3-<64hex>.blob`.
pub const OBJECT_PREFIX: &str = "b3-";
/// Filename extension on a pooled object.
pub const OBJECT_EXT: &str = "blob";

/// Why a snapshot was taken. Tags the auto-snapshot taken before a revert (Decision ⑭)
/// so the UI can label it and retention (⑯) can protect it from a prune.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotReason {
    /// A user-initiated "Snapshot now".
    Manual,
    /// The undo point auto-captured before a revert (Decision ⑭). Never auto-pruned.
    PreRestore,
}

impl SnapshotReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::PreRestore => "pre-restore",
        }
    }
}

/// One entry's blob, addressed by the content hash of its **ciphertext** (Decision ⑫).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotObject {
    /// The entry whose blob this is (the live vault stores it as `{entry_ulid}.blob`).
    pub entry_id: String,
    /// Lowercase-hex BLAKE3-256 of the on-disk blob bytes (`nonce || ciphertext`) — the
    /// CIPHERTEXT, never the plaintext. Names the object in the pool: `objects/b3-<hash>.blob`.
    pub blake3: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// A reader that does not recognise this version MUST refuse rather than guess.
    pub format_version: u32,
    pub reason: SnapshotReason,
    /// Which VAULT this is (4.6a identity). `None` only for a pre-4.6 vault.
    pub vault_uuid: Option<String>,
    /// The `.vdb`'s own `schema_version` at snapshot time.
    pub schema_version: i32,
    /// RFC-3339 millis UTC, fixed-width (`domain::shared::format_rfc3339_millis`).
    pub created_at: String,
    /// The vault's monotonic `commit_counter` at snapshot time (drives rollback framing).
    pub commit_counter: i64,
    /// First 16 hex chars of the vault's `verify_hash` at snapshot time. Compared against
    /// the live vault to drive the ⚠ stale badge and confirm a rewrap landed (⑬).
    pub verify_hash_prefix: String,
    /// ACTIVE-entry count (Decision-M2 — `all_active()`, NOT `entries.len()`).
    pub entry_count: u64,
    pub blob_count: u64,
    /// Lowercase-hex BLAKE3-256 of the snapshot's own `vault.vdb`.
    pub vault_blake3: String,
    /// One row per blob: entry → content-addressed object. The revert path reconstructs
    /// `blobs/{entry_id}.blob` from these.
    pub objects: Vec<SnapshotObject>,
}

/// The first 16 hex chars (8 bytes) of a `verify_hash`, lowercase.
///
/// Drives the ⚠ stale badge (a snapshot whose prefix differs from the live vault failed a
/// rewrap — ⑬) and confirms a rewrap landed. A prefix, not the whole hash: enough to
/// disambiguate, nothing invertible.
#[must_use]
pub fn verify_hash_prefix(verify_hash: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(16);
    for b in verify_hash.iter().take(8) {
        write!(s, "{b:02x}").ok();
    }
    s
}
