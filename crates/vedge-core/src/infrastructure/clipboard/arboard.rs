//! OS clipboard via the cross-platform `arboard` crate.

use std::sync::Mutex;

use arboard::Clipboard;
use tracing::instrument;

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

/// Holds a single `arboard::Clipboard` handle behind a mutex so the provider
/// stays `Send + Sync` while the underlying crate is `!Sync`.
pub struct ArboardClipboardProvider {
    inner: Mutex<Clipboard>,
}

impl ArboardClipboardProvider {
    /// Initialise the OS clipboard handle. Returns `Storage::Io` if the
    /// platform clipboard is unavailable (e.g. headless Linux without X).
    pub fn new() -> Result<Self, VaultError> {
        let cb = Clipboard::new()
            .map_err(|e| VaultError::Storage(StorageError::Io(format!("clipboard init: {e}"))))?;
        Ok(Self {
            inner: Mutex::new(cb),
        })
    }
}

fn clipboard_err(op: &str, e: &arboard::Error) -> VaultError {
    VaultError::Storage(StorageError::Io(format!("clipboard {op}: {e}")))
}

impl ClipboardProvider for ArboardClipboardProvider {
    #[instrument(skip_all, fields(bytes = text.len()))]
    fn set(&self, text: &str) -> Result<(), VaultError> {
        let Ok(mut cb) = self.inner.lock() else {
            return Err(VaultError::Storage(StorageError::Io(
                "clipboard mutex poisoned".into(),
            )));
        };
        cb.set_text(text).map_err(|e| clipboard_err("set", &e))
    }

    #[instrument(skip_all)]
    fn clear(&self) -> Result<(), VaultError> {
        let Ok(mut cb) = self.inner.lock() else {
            return Err(VaultError::Storage(StorageError::Io(
                "clipboard mutex poisoned".into(),
            )));
        };
        // `arboard::Clipboard::clear` exists on 3.x; fall back to empty
        // string if it fails.
        match cb.clear() {
            Ok(()) => Ok(()),
            Err(_) => cb.set_text("").map_err(|e| clipboard_err("clear", &e)),
        }
    }
}
