//! Import DTO converters (slice 5.3b). Core ⇄ wire.
//!
//! The preview crosses **derivatives only** (Decision ⑦): the core
//! `ImportPreviewRow` is already secret-free, so this is a mechanical shape map
//! (the `RowStatus` enum flattens to a `status` string + optional message/line).

pub use vedge_ipc::{ImportActionDto, ImportFailureDto, ImportPreviewRow, ImportReportDto};

use vedge_core::{
    ImportAction, ImportPreviewRow as CoreImportPreviewRow, ImportReport, RowAction, RowStatus,
};

/// Core preview row → wire DTO. Flattens `RowStatus` into `status` +
/// `status_message` + `status_line`.
#[must_use]
pub fn preview_row_to_dto(r: &CoreImportPreviewRow) -> ImportPreviewRow {
    let (status, status_message, status_line) = match &r.status {
        RowStatus::Ok => ("ok", None, None),
        RowStatus::Warning(m) => ("warning", Some(m.clone()), None),
        RowStatus::Error { message, line } => ("error", Some(message.clone()), *line),
    };
    ImportPreviewRow {
        row_id: r.row_id,
        entry_type: r.entry_type.clone(),
        name: r.name.clone(),
        username: r.username.clone(),
        url: r.url.clone(),
        tags: r.tags.clone(),
        has_password: r.has_password,
        status: status.to_owned(),
        status_message,
        status_line,
        duplicate_of: r.duplicate_of,
    }
}

/// Wire action → core action (`import = false` → Skip).
#[must_use]
pub const fn action_from_dto(a: &ImportActionDto) -> ImportAction {
    ImportAction {
        row_id: a.row_id,
        action: if a.import {
            RowAction::Import
        } else {
            RowAction::Skip
        },
    }
}

/// Core report → wire DTO.
#[must_use]
pub fn report_to_dto(r: ImportReport) -> ImportReportDto {
    let ImportReport {
        imported,
        skipped,
        failed,
    } = r;
    ImportReportDto {
        imported,
        skipped,
        failed: failed
            .into_iter()
            .map(|f| ImportFailureDto {
                row_id: f.row_id,
                message: f.message,
            })
            .collect(),
    }
}
