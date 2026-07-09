//! Native file-dialog wrappers over the Tauri dialog plugin
//! (`window.__TAURI__.dialog`).
//!
//! Only the save dialog is bound for now — it returns a plain path string, so
//! there is no fs-plugin scope to worry about. The chosen path is handed to a
//! backend command (`create_vault`, `write_emergency_kit_pdf`) that does the
//! actual file work.

use std::cell::Cell;

use serde::Serialize;

use crate::api::error::ApiError;
use crate::api::tauri::{dialog_open, dialog_save};

thread_local! {
    /// Depth of in-flight native dialogs. WASM is single-threaded, so a plain
    /// `thread_local` counter is race-free. A native OS file dialog steals the
    /// window's focus, which fires the DOM `blur` event — auto-lock must not
    /// treat *our own* dialog as the user leaving the app.
    /// See `crate::features::vault::auto_lock`.
    static IN_DIALOG: Cell<u32> = const { Cell::new(0) };
}

/// True while a native file dialog opened by [`open`] / [`save`] is on screen.
/// Read (untracked, non-reactive) by the auto-lock blur guard — safe to call
/// from an owner-less JS callback because it touches only a `thread_local`.
#[must_use]
pub fn dialog_in_progress() -> bool {
    IN_DIALOG.with(|c| c.get() > 0)
}

/// RAII marker: bumps the in-dialog counter for its lifetime and clears it on
/// every exit path — including a dropped/cancelled future.
struct DialogGuard;

impl DialogGuard {
    fn new() -> Self {
        IN_DIALOG.with(|c| c.set(c.get().wrapping_add(1)));
        Self
    }
}

impl Drop for DialogGuard {
    fn drop(&mut self) {
        IN_DIALOG.with(|c| c.set(c.get().wrapping_sub(1)));
    }
}

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
    let _guard = DialogGuard::new();
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
    let _guard = DialogGuard::new();
    let args = serde_wasm_bindgen::to_value(options).map_err(ApiError::serialize)?;
    let raw = dialog_open(args).await.map_err(ApiError::from_rejection)?;
    // Single-select resolves to a path string or `null` (cancelled).
    serde_wasm_bindgen::from_value::<Option<String>>(raw).map_err(ApiError::deserialize)
}

#[cfg(test)]
mod tests {
    use super::{
        DialogFilter, DialogGuard, OpenDialogOptions, SaveDialogOptions, dialog_in_progress,
    };

    #[test]
    fn dialog_guard_tracks_in_progress_and_nests() {
        assert!(!dialog_in_progress());
        {
            let _outer = DialogGuard::new();
            assert!(dialog_in_progress());
            {
                let _inner = DialogGuard::new();
                assert!(dialog_in_progress());
            }
            // Inner dropped; outer still holds the flag.
            assert!(dialog_in_progress());
        }
        // Both dropped; back to clear.
        assert!(!dialog_in_progress());
    }

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
