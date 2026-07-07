pub mod change_password;
pub mod copy_field;
pub mod create_entry;
pub mod create_vault;
pub mod document_ops;
pub mod entry_history;
pub mod export_emergency_kit;
pub mod get_entry;
pub mod hard_delete_entry;
pub mod lock_vault;
pub mod move_entry;
pub mod queries;
pub mod recover_vault;
pub mod refs;
pub mod restore_entry;
pub mod run_maintenance;
pub mod set_favorite;
pub mod set_sort_order;
pub mod set_tags;
pub mod soft_delete_entry;
pub mod tag_ops;
pub mod unlock_vault;
pub mod update_entry;

pub use change_password::{ChangePasswordInput, change_password};
pub use copy_field::{CopyFieldInput, FieldSelector, copy_field};
pub use create_entry::{CreateEntryInput, CreateEntryOutput, create_entry};
pub use create_vault::{CreateVault, CreateVaultInput, CreateVaultOutput};
pub use document_ops::{
    DOCUMENT_SIZE_LIMIT_BYTES, ImportDocumentInput, export_document, import_document,
};
pub use entry_history::{
    CopyHistoryFieldInput, HISTORY_MAX_VERSIONS, HistoryVersion, changed_fields,
    copy_history_field, get_history_value, list_history, restore_from_history,
};
pub use export_emergency_kit::{ExportEmergencyKitInput, export_emergency_kit};
pub use get_entry::{GetEntryInput, get_entry};
pub use hard_delete_entry::hard_delete_entry;
pub use lock_vault::lock_vault;
pub use move_entry::move_entry;
pub use queries::{
    entries_by_domain, entries_by_folder, entries_by_tag, list_active_entries, list_tags,
    list_trashed_entries, search_entries,
};
pub use recover_vault::{RecoverVaultInput, RecoveryOutcome, recover_vault};
pub use restore_entry::restore_entry;
pub use run_maintenance::{MaintenanceReport, run_maintenance};
pub use set_favorite::set_favorite;
pub use set_sort_order::set_sort_order;
pub use set_tags::set_tags;
pub use soft_delete_entry::soft_delete_entry;
pub use tag_ops::{create_tag, delete_tag, normalize_tag_name, rename_tag};
pub use unlock_vault::{UnlockVault, UnlockVaultInput};
pub use update_entry::{UpdateEntryInput, update_entry};
