//! Snapshot DTO converters (slice 5.2.1). Core reports / store entries → wire DTOs.

pub use vedge_ipc::{RevertReportDto, SnapshotDto, SnapshotReportDto};

use vedge_core::infrastructure::snapshot::store::SnapshotEntry;
use vedge_core::{RevertReport, SnapshotReport};

#[must_use]
pub fn snapshot_report_to_dto(r: SnapshotReport) -> SnapshotReportDto {
    SnapshotReportDto {
        id: r.id,
        created_at: r.created_at,
        reason: r.reason.as_str().to_owned(),
        entry_count: r.entry_count,
        blob_count: r.blob_count,
    }
}

#[must_use]
pub fn revert_report_to_dto(r: RevertReport) -> RevertReportDto {
    RevertReportDto {
        vault_uuid: r.vault_uuid,
        entry_count: r.entry_count,
        blob_count: r.blob_count,
        reverted_at: r.reverted_at,
    }
}

/// A store entry → wire DTO. `live_prefix` is the live vault's `verify_hash` prefix (if
/// readable); a snapshot whose prefix differs failed a rewrap and is badged stale.
#[must_use]
pub fn snapshot_entry_to_dto(entry: &SnapshotEntry, live_prefix: Option<&str>) -> SnapshotDto {
    let stale_credential = live_prefix.is_some_and(|lp| entry.manifest.verify_hash_prefix != lp);
    SnapshotDto {
        id: entry.id(),
        created_at: entry.manifest.created_at.clone(),
        reason: entry.manifest.reason.as_str().to_owned(),
        entry_count: entry.manifest.entry_count,
        blob_count: entry.manifest.blob_count,
        stale_credential,
    }
}
