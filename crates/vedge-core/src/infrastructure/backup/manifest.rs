//! The backup manifest — the one new on-disk format slice 5.2 introduces.
//!
//! Lives inside the tar as `manifest.json`. Self-describing (`format_version`):
//! a reader that sees a version it does not know MUST refuse rather than guess.
//! The `files[]` list (per-member BLAKE3) is the mandatory integrity gate restore
//! verifies before touching any live file. It never serializes an `EntryPayload`
//! and never touches the `A3-` Secret Key — backup inherits the `.vdb`'s
//! compatibility story, it does not create a new one.

use serde::{Deserialize, Serialize};

/// Bumped ONLY on a breaking change to the archive/manifest shape.
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// The manifest member name inside the tar.
pub const MANIFEST_NAME: &str = "manifest.json";
/// The `.vdb` snapshot member name inside the tar.
pub const VAULT_MEMBER: &str = "vault.vdb";
/// Prefix for blob members inside the tar (`blobs/{entry_ulid}.blob`).
pub const BLOBS_PREFIX: &str = "blobs/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// A reader that does not recognise this version MUST refuse (never guess).
    pub format_version: u32,
    /// Which VAULT this is (4.6a identity). `None` only for a pre-4.6 vault backed
    /// up before its first post-4.6 unlock — unrealistic in practice (backup needs
    /// an unlocked session, and unlock backfills the uuid).
    pub vault_uuid: Option<String>,
    /// The `.vdb`'s own `schema_version` at snapshot time. Restore REFUSES a value
    /// it does not understand — the first real use of this field since Phase 1.
    pub schema_version: i32,
    /// RFC-3339 millis UTC, fixed-width (`domain::shared::format_rfc3339_millis`).
    pub created_at: String,
    pub entry_count: u64,
    pub blob_count: u64,
    /// The vault's monotonic `commit_counter` at snapshot time (slice 5.2c). Lets
    /// `inspect_backup` frame a restore as a rollback ("restoring to state N; this
    /// device is at M"). `#[serde(default)]` so pre-5.2c archives (which lack the
    /// field) deserialize to 0 — additive, so `format_version` stays 1.
    #[serde(default)]
    pub commit_counter: i64,
    /// One row per tar member EXCEPT the manifest itself. The mandatory integrity
    /// gate: restore verifies every extracted file against this and refuses on any
    /// mismatch, naming the offending member.
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Member name inside the tar: `vault.vdb` or `blobs/{entry_ulid}.blob`.
    pub name: String,
    pub size: u64,
    /// Lowercase-hex BLAKE3-256 of the member's bytes.
    pub blake3: String,
}
