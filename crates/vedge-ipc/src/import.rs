//! Import wire DTOs (slice 5.3b).
//!
//! 🔴 Decision ⑦ — **derivatives only** cross this boundary. A preview row carries
//! `has_password: bool` (the 4.2 `has_totp` shape), never the secret. The commit
//! happens entirely in Rust; the frontend only picks rows by `row_id`.

use serde::{Deserialize, Serialize};

/// One import preview row. `has_password` is a non-invertible presence flag; the
/// password itself never crosses. `status` is `"ok"` | `"warning"` | `"error"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewRow {
    pub row_id: u32,
    pub entry_type: String,
    pub name: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub has_password: bool,
    pub status: String,
    /// Human message for a warning/error badge.
    #[serde(default)]
    pub status_message: Option<String>,
    /// The 1-based source line for a malformed CSV row.
    #[serde(default)]
    pub status_line: Option<u64>,
    /// The `row_id` of an earlier row this one likely duplicates (never
    /// auto-merged — advisory only).
    #[serde(default)]
    pub duplicate_of: Option<u32>,
}

/// One user decision for a row: import it, or skip it. `import = false` → skip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportActionDto {
    pub row_id: u32,
    pub import: bool,
}

/// A row that failed to commit (best-effort import — the batch is not aborted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFailureDto {
    pub row_id: u32,
    pub message: String,
}

/// The result of committing an import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReportDto {
    pub imported: u64,
    pub skipped: u64,
    #[serde(default)]
    pub failed: Vec<ImportFailureDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔴 Decision ⑦ — the preview must carry no secret. A serialized row has a
    /// `has_password` flag but no `password` (nor any other secret) key.
    #[test]
    fn preview_row_carries_no_secret() {
        let row = ImportPreviewRow {
            row_id: 0,
            entry_type: "login".to_owned(),
            name: "GitHub".to_owned(),
            username: Some("octocat".to_owned()),
            url: Some("https://github.com".to_owned()),
            tags: vec!["work".to_owned()],
            has_password: true,
            status: "ok".to_owned(),
            status_message: None,
            status_line: None,
            duplicate_of: None,
        };
        let v = serde_json::to_value(&row).expect("serialize");
        assert!(v.get("password").is_none(), "no password key may cross");
        assert!(v.get("totp_secret").is_none());
        assert!(v.get("secret").is_none());
        assert_eq!(v.get("has_password"), Some(&serde_json::Value::Bool(true)));
    }
}
