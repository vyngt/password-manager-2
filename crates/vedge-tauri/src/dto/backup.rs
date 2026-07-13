//! Backup/restore DTO converters (slice 5.2 / 5.2b). Core reports → wire DTOs.

pub use vedge_ipc::{BackupPreviewDto, BackupReportDto, RestoreReportDto};

use vedge_core::{BackupPreview, BackupReport, RestoreReport};

#[must_use]
pub fn backup_report_to_dto(r: BackupReport) -> BackupReportDto {
    BackupReportDto {
        archive_path: r.archive_path.to_string_lossy().into_owned(),
        archive_bytes: r.archive_bytes,
        archive_blake3: r.archive_blake3,
        entry_count: r.entry_count,
        blob_count: r.blob_count,
        created_at: r.created_at,
    }
}

#[must_use]
pub fn backup_preview_to_dto(p: BackupPreview) -> BackupPreviewDto {
    BackupPreviewDto {
        format_version: p.format_version,
        backup_vault_uuid: p.backup_vault_uuid,
        backup_schema_version: p.backup_schema_version,
        created_at: p.created_at,
        backup_entry_count: p.backup_entry_count,
        backup_blob_count: p.backup_blob_count,
        target_uuid: p.target_uuid,
        target_entry_count: p.target_entry_count,
        target_last_unlocked_at: p.target_last_unlocked_at,
        target_commit_counter: p.target_commit_counter,
        uuid_mismatch: p.uuid_mismatch,
        unknown_format: p.unknown_format,
        unknown_schema: p.unknown_schema,
        target_unreadable: p.target_unreadable,
        rollback_delta: p.rollback_delta,
    }
}

#[must_use]
pub fn restore_report_to_dto(r: RestoreReport) -> RestoreReportDto {
    RestoreReportDto {
        vault_uuid: r.vault_uuid,
        entry_count: r.entry_count,
        blob_count: r.blob_count,
        restored_at: r.restored_at,
    }
}
