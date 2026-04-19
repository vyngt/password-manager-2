pub mod change_password;
pub mod copy_field;
pub mod create_entry;
pub mod document_ops;
pub mod export_emergency_kit;
pub mod hard_delete_entry;
pub mod lock_vault;
pub mod move_entry;
pub mod queries;
pub mod recover_vault;
pub mod refs;
pub mod restore_entry;
pub mod run_maintenance;
pub mod soft_delete_entry;
pub mod tag_ops;
pub mod unlock_vault;
pub mod update_entry;

pub use change_password::{change_password, ChangePasswordInput};
pub use copy_field::{copy_field, CopyFieldInput, FieldSelector};
pub use create_entry::{create_entry, CreateEntryInput, CreateEntryOutput};
pub use document_ops::{
    export_document, import_document, ImportDocumentInput, DOCUMENT_SIZE_LIMIT_BYTES,
};
pub use export_emergency_kit::{export_emergency_kit, ExportEmergencyKitInput};
pub use hard_delete_entry::hard_delete_entry;
pub use lock_vault::lock_vault;
pub use move_entry::move_entry;
pub use queries::{
    entries_by_domain, entries_by_folder, entries_by_tag, list_active_entries, list_tags,
    list_trashed_entries, search_entries,
};
pub use recover_vault::{recover_vault, RecoverVaultInput, RecoveryOutcome};
pub use restore_entry::restore_entry;
pub use run_maintenance::{run_maintenance, MaintenanceReport};
pub use soft_delete_entry::soft_delete_entry;
pub use tag_ops::{create_tag, delete_tag, normalize_tag_name, rename_tag};
pub use unlock_vault::{UnlockVault, UnlockVaultInput};
pub use update_entry::{update_entry, UpdateEntryInput};

