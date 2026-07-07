use async_trait::async_trait;

use crate::domain::shared::{EntryId, TagId, Timestamp};
use crate::domain::vault::entities::{AuditEvent, EntryHistoryRow, EntryRow, TagRow, VaultConfig};
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

    /// Atomic re-wrap during `ChangePassword`.
    ///
    /// Updates every listed entry's `dek_wrapped` **and** the vault config
    /// (which carries the new `verify_hash` / `kdf_params` /
    /// `last_unlocked_at`) as a single transaction. If any step fails the
    /// whole batch rolls back, so the vault never lands in a "half the
    /// DEKs are under the new KEK, the other half under the old KEK"
    /// state — which would be unrecoverable by either password.
    async fn rewrap_all_deks(
        &self,
        updates: &[(EntryId, [u8; 40])],
        new_config: &VaultConfig,
    ) -> Result<(), VaultError>;
}
