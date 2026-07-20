/// Human-readable byte size using 1024-based units.
///
/// ```
/// # use vedge_ui::utils::format::format_bytes;
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1024), "1 KB");
/// assert_eq!(format_bytes(10_485_760), "10.0 MB");
/// ```
pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.1} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.0} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}

/// Display a float without trailing `.0` when it's a whole number.
///
/// ```
/// # use vedge_ui::utils::format::format_float_display;
/// assert_eq!(format_float_display(3.0), "3");
/// assert_eq!(format_float_display(3.14), "3.14");
/// ```
pub fn format_float_display(val: f64) -> String {
    if val.is_finite() && val == val.trunc() {
        format!("{}", val as i64)
    } else {
        format!("{}", val)
    }
}
