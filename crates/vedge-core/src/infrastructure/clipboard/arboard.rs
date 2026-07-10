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
        // Every value handed to `set` is a secret (the only production caller is
        // `place_text_on_clipboard`), so the platform *exclusion hints* are
        // applied unconditionally — no per-call flag. They stop the value from
        // being *captured* at set-time by clipboard history / cloud sync / third-
        // party managers, which the 30 s clear timer can't undo. arboard 3.6
        // exposes each hint on its `Set` builder under `default-features = false`
        // (target-cfg-gated, not feature-gated), so no shim is needed.
        let builder = cb.set();
        // Windows: `ExcludeClipboardContentFromMonitorProcessing` — the documented
        // superset that also suppresses Win+V history and cloud clipboard upload.
        #[cfg(target_os = "windows")]
        let builder = {
            use arboard::SetExtWindows as _;
            builder.exclude_from_monitoring()
        };
        // macOS: writes the `org.nspasteboard.ConcealedType` pasteboard type
        // honored by well-behaved clipboard managers.
        #[cfg(target_os = "macos")]
        let builder = {
            use arboard::SetExtApple as _;
            builder.exclude_from_history()
        };
        // Linux/X11: advertises the concealment targets (incl.
        // `x-kde-passwordManagerHint`) respected by Klipper and others. Wayland
        // has no universal hint and arboard's Wayland exclusion is behind the
        // `wayland-data-control` feature (off here) — documented residual risk.
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        let builder = {
            use arboard::SetExtLinux as _;
            builder.exclude_from_history()
        };
        builder.text(text).map_err(|e| clipboard_err("set", &e))
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
