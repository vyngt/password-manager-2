use std::path::Path;

use async_trait::async_trait;

use crate::domain::shared::{EntryId, TagId, Timestamp};
use crate::domain::vault::entities::{
    AuditEvent, AuditPage, AuditQuery, EntryHistoryRow, EntryRow, TagRow, VaultConfig,
};
use crate::domain::vault::errors::VaultError;

#[async_trait]
pub trait VaultRepository: Send + Sync {
    // vault_config (exactly one row, id = "default")
    async fn load_config(&self) -> Result<VaultConfig, VaultError>;
    async fn save_config(&self, config: &VaultConfig) -> Result<(), VaultError>;

    // entries — raw encrypted rows
    async fn insert_entry(&self, row: &EntryRow) -> Result<(), VaultError>;
    async fn update_entry(&self, row: &EntryRow) -> Result<(), VaultError>;
    async fn get_entry(&self, id: &EntryId) -> Result<EntryRow, VaultError>;
    async fn all_entries(&self) -> Result<Vec<EntryRow>, VaultError>;
    async fn soft_delete_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;
    async fn restore_entry(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;
    async fn hard_delete_entry(&self, id: &EntryId) -> Result<(), VaultError>;
    async fn hard_delete_trashed_before(&self, cutoff: Timestamp) -> Result<u64, VaultError>;
    async fn update_accessed_at(&self, id: &EntryId, when: Timestamp) -> Result<(), VaultError>;

    // tags — raw encrypted rows
    async fn insert_tag(&self, row: &TagRow) -> Result<(), VaultError>;
    async fn update_tag(&self, row: &TagRow) -> Result<(), VaultError>;
    async fn get_tag(&self, id: &TagId) -> Result<TagRow, VaultError>;
    async fn all_tags(&self) -> Result<Vec<TagRow>, VaultError>;
    async fn delete_tag(&self, id: &TagId) -> Result<(), VaultError>;

    // entry_history — encrypted prior-version snapshots (no key material)
    async fn insert_history(&self, row: &EntryHistoryRow) -> Result<(), VaultError>;
    /// Snapshots for one entry, newest-first (by version).
    async fn list_history(&self, entry_id: &EntryId) -> Result<Vec<EntryHistoryRow>, VaultError>;
    async fn get_history(&self, id: &str) -> Result<EntryHistoryRow, VaultError>;
    async fn delete_history_for_entry(&self, entry_id: &EntryId) -> Result<u64, VaultError>;
    /// Keep the `keep` newest snapshots for an entry; delete the rest. Returns
    /// how many were pruned.
    async fn prune_history_keep(&self, entry_id: &EntryId, keep: usize) -> Result<u64, VaultError>;

    // audit_log
    async fn append_audit(&self, event: &AuditEvent) -> Result<(), VaultError>;
    async fn recent_audit(&self, limit: u32) -> Result<Vec<AuditEvent>, VaultError>;
    async fn audit_by_entry(
        &self,
        entry_id: &EntryId,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, VaultError>;
    async fn delete_audit_before(&self, cutoff: Timestamp) -> Result<u64, VaultError>;
    /// A filtered, paged read over `audit_log`. All predicates are applied in
    /// SQL; `total` is the full match count ignoring limit/offset. A stored
    /// `action` string this build cannot parse is skipped + warned, not
    /// propagated as an error (forward-compat for actions added in later slices).
    async fn query_audit(&self, q: &AuditQuery) -> Result<AuditPage, VaultError>;

    /// Atomic re-wrap during `ChangePassword`.
    ///
    /// Updates every listed entry's `dek_wrapped`, every listed **tag** row (its
    /// `nonce` / `ciphertext` / `dek_wrapped` — a re-wrapped DEK leaves the
    /// ciphertext unchanged, a migrated legacy tag replaces all three), **and** the
    /// vault config (which carries the new `verify_hash` / `kdf_params` /
    /// `last_unlocked_at`) as a single transaction. If any step fails the whole batch
    /// rolls back, so the vault never lands in a "half the keys are under the new KEK,
    /// the other half under the old KEK" state — which would be unrecoverable by either
    /// password. 🔴 Tags were added here in slice 5.6.0: they used to be sealed directly
    /// under the KEK and were silently skipped, which bricked any tagged vault on a
    /// password change (and every snapshot, since `rewrap_snapshots` funnels through here).
    async fn rewrap_all_deks(
        &self,
        updates: &[(EntryId, [u8; 40])],
        tag_updates: &[TagRow],
        new_config: &VaultConfig,
    ) -> Result<(), VaultError>;

    /// Write a live-safe, consistent snapshot of the vault DB to `dest` (slice
    /// 5.2 backup). Uses `SQLite`'s `VACUUM INTO`: `WAL`-aware, non-blocking, no
    /// forced lock. **`dest` must not already exist** and this cannot run inside a
    /// transaction — callers vacuum into a fresh path in a temp dir. The snapshot
    /// is logically identical but defragmented (not byte-identical) to the source.
    async fn vacuum_into(&self, dest: &Path) -> Result<(), VaultError>;

    /// Close the underlying DB pool, releasing the OS file handle SYNCHRONOUSLY (slice 5.2.1).
    ///
    /// 🔴 sqlx's pool close is async on `Drop`, so simply dropping a session leaves the
    /// `.vdb`/`-wal`/`-shm` handle open for a while afterwards. A file operation that follows a
    /// lock — an in-place `revert_to_snapshot`, a `.vbk` restore — would then race that
    /// lingering handle and fail to rename the home on Windows (`os error 5`/`32`). Calling
    /// this at lock time makes the handle GONE by the time the lock returns, so the race
    /// cannot happen — deterministic, not a timing retry. Idempotent.
    async fn close(&self);

    /// Set `vault_config.last_snapshot_at` in a targeted update (slice 5.2.1). Kept out
    /// of the config-upsert `update_columns`, so this is the only writer — a stale
    /// `save_config` can never clobber it. 🔴 Never `last_backup_at` (Decision ⑧).
    async fn touch_last_snapshot_at(&self, at: Timestamp) -> Result<(), VaultError>;

    /// Set `vault_config.last_backup_at` in a targeted update (slice 5.2.1) — written only
    /// by `backup_vault` when a `.vbk` is produced. Separate from `last_snapshot_at`.
    async fn touch_last_backup_at(&self, at: Timestamp) -> Result<(), VaultError>;

    /// Set `vault_config.recovery_slot` in a targeted update (slice 5.7) — recovery
    /// enroll. Kept out of the config-upsert `update_columns` so a stale `save_config`
    /// cannot clobber it; a KEK change nulls it in-txn via `rewrap_all_deks`.
    async fn set_recovery_slot(&self, slot: &[u8; 40]) -> Result<(), VaultError>;

    /// Null `vault_config.recovery_slot` in a targeted update (slice 5.7) — revoke, and
    /// the null-in-snapshot at capture (④). Idempotent.
    async fn clear_recovery_slot(&self) -> Result<(), VaultError>;

    /// Set `vault_config.last_password_change_at` in a targeted update (slice 5.9 ③). Kept
    /// out of `save_config`'s `update_columns`, so `rekey_vault` (which persists config via
    /// `save_config` on the staged repo) uses this instead of the in-memory config value.
    async fn touch_last_password_change_at(&self, at: Timestamp) -> Result<(), VaultError>;

    /// Set `vault_config.last_secret_key_rotation_at` in a targeted update (slice 5.9 ③) —
    /// written only on a real Secret-Key change (not a recovery reset).
    async fn touch_last_secret_key_rotation_at(&self, at: Timestamp) -> Result<(), VaultError>;

    /// Null `vault_config.last_snapshot_at` in a targeted update (slice 5.9 ④) — `rekey_vault`
    /// retires the snapshot store, so the "last snapshot" timestamp must not claim one exists.
    async fn clear_last_snapshot_at(&self) -> Result<(), VaultError>;
}
