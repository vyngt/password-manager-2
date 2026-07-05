//! Native file-dialog wrappers over the Tauri dialog plugin
//! (`window.__TAURI__.dialog`).
//!
//! Only the save dialog is bound for now — it returns a plain path string, so
//! there is no fs-plugin scope to worry about. The chosen path is handed to a
//! backend command (`create_vault`, `write_emergency_kit_pdf`) that does the
//! actual file work.

use serde::Serialize;

use crate::api::error::ApiError;
use crate::api::tauri::{dialog_open, dialog_save};

#[derive(Debug, Clone, Serialize)]
pub struct DialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<DialogFilter>,
}

/// Show a native save-file dialog. Returns the chosen path, or `None` when the
/// user cancels.
pub async fn save(options: &SaveDialogOptions) -> Result<Option<String>, ApiError> {
    let args = serde_wasm_bindgen::to_value(options).map_err(ApiError::serialize)?;
    let raw = dialog_save(args).await.map_err(ApiError::from_rejection)?;
    // Resolves to a path string or `null` (cancelled).
    serde_wasm_bindgen::from_value::<Option<String>>(raw).map_err(ApiError::deserialize)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<DialogFilter>,
}

/// Show a native open-file dialog (single-select). Returns the chosen path, or
/// `None` when the user cancels.
pub async fn open(options: &OpenDialogOptions) -> Result<Option<String>, ApiError> {
    let args = serde_wasm_bindgen::to_value(options).map_err(ApiError::serialize)?;
    let raw = dialog_open(args).await.map_err(ApiError::from_rejection)?;
    // Single-select resolves to a path string or `null` (cancelled).
    serde_wasm_bindgen::from_value::<Option<String>>(raw).map_err(ApiError::deserialize)
}

#[cfg(test)]
mod tests {
    use super::{DialogFilter, OpenDialogOptions, SaveDialogOptions};

    #[test]
    fn open_options_serialize_camel_case_and_omit_empty() {
        let opts = OpenDialogOptions {
            title: Some("Open vault".to_string()),
            filters: vec![DialogFilter {
                name: "VEdge Vault".to_string(),
                extensions: vec!["vdb".to_string()],
            }],
        };
        let v = serde_json::to_value(&opts).unwrap();
        assert_eq!(v["title"], "Open vault");
        assert_eq!(v["filters"][0]["extensions"][0], "vdb");

        let empty = serde_json::to_value(OpenDialogOptions::default()).unwrap();
        assert!(empty.get("title").is_none());
        assert!(empty.get("filters").is_none());
    }

    #[test]
    fn save_options_serialize_camel_case() {
        let opts = SaveDialogOptions {
            title: Some("Save".to_string()),
            default_path: Some("kit.pdf".to_string()),
            filters: vec![DialogFilter {
                name: "PDF".to_string(),
                extensions: vec!["pdf".to_string()],
            }],
        };
        let v = serde_json::to_value(&opts).unwrap();
        assert_eq!(v["defaultPath"], "kit.pdf");
        assert_eq!(v["title"], "Save");
        assert_eq!(v["filters"][0]["name"], "PDF");
        assert_eq!(v["filters"][0]["extensions"][0], "pdf");
    }

    #[test]
    fn save_options_omit_none_and_empty() {
        let v = serde_json::to_value(SaveDialogOptions::default()).unwrap();
        assert!(v.get("title").is_none());
        assert!(v.get("defaultPath").is_none());
        assert!(v.get("filters").is_none());
    }
}
