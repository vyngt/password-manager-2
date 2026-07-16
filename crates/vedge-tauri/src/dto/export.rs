//! Export DTO converter (slice 5.3a). Core report → wire DTO.

pub use vedge_ipc::ExportReportDto;

use vedge_core::ExportReport;

#[must_use]
pub fn export_report_to_dto(r: ExportReport) -> ExportReportDto {
    // Destructure to consume `r` by value (it is not `Copy`).
    let ExportReport {
        dest,
        entry_count,
        encrypted,
    } = r;
    ExportReportDto {
        dest_path: dest.to_string_lossy().into_owned(),
        entry_count,
        encrypted,
    }
}
