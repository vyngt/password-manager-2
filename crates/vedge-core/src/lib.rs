#![cfg_attr(test, allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::float_arithmetic,
    clippy::needless_pass_by_value,
    dead_code,
))]

pub mod application;
pub mod domain;
pub mod infrastructure;

// Convenience re-exports — the full public surface lives under the three
// top-level modules; this block lets shells write `use vedge_core::…` for
// the things they'll reach for most often.

pub use application::vault::session::VaultSession;
pub use application::vault::use_cases::{
    change_password, copy_field, create_entry, create_tag, delete_tag, export_document,
    hard_delete_entry, import_document, lock_vault, move_entry, normalize_tag_name,
    rename_tag, restore_entry, run_maintenance, soft_delete_entry, update_entry,
    ChangePasswordInput, CopyFieldInput, CreateEntryInput, CreateEntryOutput, FieldSelector,
    ImportDocumentInput, MaintenanceReport, UnlockVault, UnlockVaultInput, UpdateEntryInput,
    DOCUMENT_SIZE_LIMIT_BYTES,
};
pub use domain::vault::index::{IndexEntry, TagMeta, VaultIndex};
