//! Backup DTO converters (slices 5.2 / 5.2.2). Core reports → wire DTOs.

pub use vedge_ipc::{
    BackupPreviewDto, BackupReportDto, BackupStatusDto, ConvertVaultResultDto,
    DeleteVaultReportDto, OpenBackupReportDto, ReplaceReportDto, TargetStateDto, VaultDetailsDto,
};

use vedge_core::{BackupPreview, BackupReport, OpenBackupReport, ReplaceReport, TargetStateKind};

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
const fn target_state_to_dto(s: TargetStateKind) -> TargetStateDto {
    match s {
        TargetStateKind::Missing => TargetStateDto::Missing,
        TargetStateKind::Unreadable => TargetStateDto::Unreadable,
        TargetStateKind::Readable => TargetStateDto::Readable,
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
        target_state: target_state_to_dto(p.target_state),
        target_uuid: p.target_uuid,
        target_entry_count: p.target_entry_count,
        target_last_unlocked_at: p.target_last_unlocked_at,
        target_commit_counter: p.target_commit_counter,
        uuid_mismatch: p.uuid_mismatch,
        unknown_format: p.unknown_format,
        unknown_schema: p.unknown_schema,
        dest_occupied: p.dest_occupied,
        rollback_delta: p.rollback_delta,
        credentials_differ: p.credentials_differ,
        duplicate_of: p.duplicate_of.map(|d| d.to_string_lossy().into_owned()),
    }
}

#[must_use]
pub fn open_backup_report_to_dto(r: OpenBackupReport) -> OpenBackupReportDto {
    OpenBackupReportDto {
        home: r.home.to_string_lossy().into_owned(),
        vault_uuid: r.vault_uuid,
        duplicate_of: r.duplicate_of.map(|d| d.to_string_lossy().into_owned()),
        fresh_uuid: r.fresh_uuid,
        secret_key_copied: r.secret_key_copied,
        entry_count: r.entry_count,
        blob_count: r.blob_count,
        opened_at: r.opened_at,
    }
}

#[must_use]
pub fn replace_report_to_dto(r: ReplaceReport) -> ReplaceReportDto {
    ReplaceReportDto {
        vault_uuid: r.vault_uuid,
        entry_count: r.entry_count,
        blob_count: r.blob_count,
        restored_at: r.restored_at,
        undo_snapshot_id: r.undo_snapshot_id,
    }
}
