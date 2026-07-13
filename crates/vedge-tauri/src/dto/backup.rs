//! Backup DTO converter (slice 5.2). Core `BackupReport` → wire `BackupReportDto`.

pub use vedge_ipc::BackupReportDto;

use vedge_core::BackupReport;

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
