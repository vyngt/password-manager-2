pub mod backup_vault;
pub mod biometric;
pub mod change_password;
pub mod change_password_after_recovery;
pub mod copy_field;
pub mod create_entry;
pub mod create_snapshot;
pub mod create_vault;
pub mod credential_common;
pub mod delete_vault;
pub mod document_ops;
pub mod enroll_recovery_key;
pub mod entry_history;
pub mod env_vars;
pub mod export_emergency_kit;
pub mod export_entries;
pub mod get_entry;
pub mod hard_delete_entry;
pub mod import_entries;
pub mod inspect_backup;
pub mod list_audit;
pub mod lock_vault;
pub mod move_entry;
pub mod open_backup;
pub mod queries;
pub mod reauthenticate;
pub mod refs;
pub mod rekey_vault;
pub mod replace_vault;
pub mod restore_entry;
pub mod reveal_field;
pub mod reveal_totp;
pub mod revert_to_snapshot;
pub mod revoke_recovery_key;
pub mod rewrap_snapshots;
pub mod run_maintenance;
pub mod scan_health;
pub mod set_favorite;
pub mod set_sort_order;
pub mod set_tags;
pub mod soft_delete_entry;
pub mod tag_crypto;
pub mod tag_ops;
pub mod unlock_vault;
pub mod unlock_with_secret_key;
pub mod update_entry;

pub use backup_vault::{BackupReport, BackupStatus, BackupVaultInput, backup_status, backup_vault};
pub use biometric::enroll_biometric;
pub use change_password::{ChangePasswordInput, change_password};
pub use change_password_after_recovery::change_password_after_recovery;
pub use copy_field::{CopyFieldInput, FieldSelector, copy_field, place_text_on_clipboard};
pub use create_entry::{CreateEntryInput, CreateEntryOutput, create_entry};
pub use create_snapshot::{SnapshotReport, create_snapshot};
pub use create_vault::{CreateVault, CreateVaultInput, CreateVaultOutput};
pub use delete_vault::{DeleteVaultInput, DeleteVaultReport, delete_vault};
pub use document_ops::{
    DOCUMENT_SIZE_LIMIT_BYTES, ImportDocumentInput, export_document, import_document,
};
pub use enroll_recovery_key::{EnrollRecoveryOutcome, enroll_recovery_key};
pub use entry_history::{
    CopyHistoryFieldInput, HISTORY_MAX_VERSIONS, HistoryVersion, RevealHistoryFieldInput,
    changed_fields, copy_history_field, get_history_value, list_history, restore_from_history,
    reveal_history_field,
};
pub use env_vars::{CopyEnvVarsInput, RevealEnvVarsInput, copy_env_vars, reveal_env_vars};
pub use export_emergency_kit::{ExportEmergencyKitInput, export_emergency_kit};
pub use export_entries::{ExportEntriesInput, ExportFormat, ExportReport, export_entries};
pub use get_entry::{GetEntryInput, get_entry};
pub use hard_delete_entry::hard_delete_entry;
pub use import_entries::{
    ImportAction, ImportFailure, ImportPreviewRow, ImportReport, ImportSession, ImportSource,
    RowAction, RowStatus, begin_import, begin_snapshot_import, cancel_import, commit_import,
};
pub use inspect_backup::{BackupPreview, InspectBackupInput, TargetStateKind, inspect_backup};
pub use list_audit::{AUDIT_PAGE_DEFAULT, AUDIT_PAGE_MAX, list_audit};
pub use lock_vault::{close_session_db, lock_vault, record_lock};
pub use move_entry::move_entry;
pub use open_backup::{OpenBackupInput, OpenBackupReport, open_backup};
pub use queries::{
    entries_by_domain, entries_by_folder, entries_by_tag, list_active_entries, list_tags,
    list_trashed_entries, search_entries,
};
pub use reauthenticate::reauthenticate_master_password;
pub use rekey_vault::{RekeyOutcome, RekeyReport, RekeyVaultInput, rekey_vault};
pub use replace_vault::{ReplaceReport, ReplaceVaultInput, replace_vault_from_backup};
pub use restore_entry::restore_entry;
pub use reveal_field::{
    RevealFieldInput, RevealRecoveryCodesInput, reveal_field, reveal_recovery_codes,
};
pub use reveal_totp::reveal_totp;
pub use revert_to_snapshot::{
    RevertReport, RevertToSnapshotInput, SeamlessRevertOutcome, revert_to_snapshot,
    revert_to_snapshot_in_session,
};
pub use revoke_recovery_key::revoke_recovery_key;
pub use run_maintenance::{MaintenanceReport, run_maintenance};
pub use scan_health::scan_health;
pub use set_favorite::set_favorite;
pub use set_sort_order::set_sort_order;
pub use set_tags::set_tags;
pub use soft_delete_entry::soft_delete_entry;
pub use tag_ops::{create_tag, delete_tag, normalize_tag_name, rename_tag};
pub use unlock_vault::{UnlockVault, UnlockVaultInput};
pub use unlock_with_secret_key::{
    SecretKeyUnlockOutcome, UnlockWithSecretKeyInput, unlock_with_secret_key,
};
pub use update_entry::{UpdateEntryInput, update_entry};
