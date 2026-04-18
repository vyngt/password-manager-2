use async_trait::async_trait;

use crate::domain::shared::{EntryId, TagId, Timestamp};
use crate::domain::vault::entities::{AuditEvent, EntryRow, TagRow, VaultConfig};
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

    // audit_log
    async fn append_audit(&self, event: &AuditEvent) -> Result<(), VaultError>;
    async fn recent_audit(&self, limit: u32) -> Result<Vec<AuditEvent>, VaultError>;
    async fn audit_by_entry(
        &self,
        entry_id: &EntryId,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, VaultError>;
    async fn delete_audit_before(&self, cutoff: Timestamp) -> Result<u64, VaultError>;
}
