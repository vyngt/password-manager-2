//! Native file-dialog wrappers over the Tauri dialog plugin
//! (`window.__TAURI__.dialog`).
//!
//! Only the save dialog is bound for now — it returns a plain path string, so
//! there is no fs-plugin scope to worry about. The chosen path is handed to a
//! backend command (`create_vault`, `write_emergency_kit_pdf`) that does the
//! actual file work.

use serde::Serialize;
use wasm_bindgen::JsValue;

use crate::api::error::ApiError;
use crate::api::tauri::dialog_save;

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
