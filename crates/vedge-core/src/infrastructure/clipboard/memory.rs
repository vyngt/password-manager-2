//! In-memory clipboard double for tests. Never touches the real OS clipboard.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

pub struct MemoryClipboardProvider {
    state: Mutex<Option<String>>,
    set_count: AtomicUsize,
}

impl MemoryClipboardProvider {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(None),
            set_count: AtomicUsize::new(0),
        }
    }

    /// Test-only hook: read back what was set. Not part of the port.
    pub fn peek(&self) -> Option<String> {
        self.state.lock().ok().and_then(|g| g.clone())
    }

    /// Test-only hook: how many times `set` was invoked. Lets a test assert a
    /// value was routed through the provider independently of whether the
    /// detached clear timer has since fired. Not part of the port.
    #[must_use]
    pub fn set_count(&self) -> usize {
        self.set_count.load(Ordering::Relaxed)
    }
}

impl Default for MemoryClipboardProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardProvider for MemoryClipboardProvider {
    fn set(&self, text: &str) -> Result<(), VaultError> {
        let Ok(mut guard) = self.state.lock() else {
            return Err(VaultError::Storage(StorageError::Io(
                "memory clipboard poisoned".into(),
            )));
        };
        *guard = Some(text.to_owned());
        self.set_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn clear(&self) -> Result<(), VaultError> {
        let Ok(mut guard) = self.state.lock() else {
            return Err(VaultError::Storage(StorageError::Io(
                "memory clipboard poisoned".into(),
            )));
        };
        *guard = None;
        Ok(())
    }
}
