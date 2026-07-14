#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::float_arithmetic,
        clippy::needless_pass_by_value,
        dead_code,
    )
)]

pub mod application;
pub mod domain;
pub mod infrastructure;

// Convenience re-exports — the full public surface lives under the three
// top-level modules; this block lets shells write `use vedge_core::…` for
// the things they'll reach for most often.

pub use application::app::use_cases::{
    ACTIVE_THEME_SETTING_KEY, AddRecentVaultInput, CreateCustomThemeInput, DEFAULT_THEME_ID,
    RecentVaultStatus, SQLITE_MAGIC, UpdateCustomThemeInput, add_recent_vault, create_custom_theme,
    delete_app_setting, delete_custom_theme, delete_extension_session, delete_known_device,
    duplicate_theme, get_app_setting, get_theme, list_app_settings, list_extension_sessions,
    list_known_devices, list_recent_vaults, list_recent_vaults_with_status, list_themes,
    remove_recent_vault, remove_stale_recents, rename_recent_vault, resolve_active_theme,
    set_active_theme, set_app_setting, touch_extension_session_last_active,
    touch_known_device_last_seen, touch_on_unlock, touch_recent_vault, update_custom_theme,
    upsert_extension_session, upsert_known_device, validate_hex_color,
};
pub use application::vault::session::VaultSession;
pub use application::vault::use_cases::{
    AUDIT_PAGE_DEFAULT, AUDIT_PAGE_MAX, BackupPreview, BackupReport, BackupVaultInput,
    ChangePasswordInput, CopyFieldInput, CopyHistoryFieldInput, CreateEntryInput,
    CreateEntryOutput, CreateVault, CreateVaultInput, CreateVaultOutput, DOCUMENT_SIZE_LIMIT_BYTES,
    ExportEmergencyKitInput, FieldSelector, GetEntryInput, HistoryVersion, ImportDocumentInput,
    InspectBackupInput, MaintenanceReport, MigrateVaultLayoutInput, MigrateVaultLayoutOutput,
    RecoverVaultInput, RecoveryOutcome, RestoreReport, RestoreVaultInput, SnapshotReport,
    UnlockVault, UnlockVaultInput, UpdateEntryInput, backup_vault, change_password, copy_field,
    copy_history_field, create_entry, create_snapshot, create_tag, delete_tag, enroll_biometric,
    entries_by_domain,
    entries_by_folder, entries_by_tag, export_document, export_emergency_kit, get_entry,
    get_history_value, hard_delete_entry, import_document, inspect_backup, list_active_entries,
    list_audit, list_history, list_tags, list_trashed_entries, lock_vault, migrate_vault_layout,
    move_entry, normalize_tag_name, place_text_on_clipboard, record_lock, recover_vault,
    rename_tag, restore_entry, restore_from_history, restore_vault, reveal_totp, run_maintenance,
    scan_health, search_entries, set_favorite, set_sort_order, set_tags, soft_delete_entry,
    update_entry,
};
pub use domain::vault::emergency_kit::EmergencyKitContent;
pub use domain::vault::entities::{AuditAction, AuditEvent, AuditPage, AuditQuery};
pub use domain::vault::health::{
    AgeConfidence, Finding, FindingKind, HealthReport, HealthScanInput, HealthSummary, SecretField,
    Severity, SkipReason, Skipped, WeakPolicy,
};
pub use domain::vault::index::{IndexEntry, TagMeta, VaultIndex};
pub use domain::vault::otpauth::{TotpEnrolment, parse_totp_input};
pub use domain::vault::recovery::{RECOVERY_FORMAT_PREFIX, format_secret_key, parse_secret_key};
pub use domain::vault::totp::{TotpAlgorithm, TotpCode, TotpParams, TotpUpdate};
pub use infrastructure::snapshot::manifest::{SnapshotManifest, SnapshotReason};
