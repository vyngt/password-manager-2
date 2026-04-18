//! In-memory clipboard double for tests. Never touches the real OS clipboard.

use std::sync::Mutex;

use crate::application::vault::ports::clipboard::ClipboardProvider;
use crate::domain::shared::StorageError;
use crate::domain::vault::errors::VaultError;

pub struct MemoryClipboardProvider {
    state: Mutex<Option<String>>,
}

impl MemoryClipboardProvider {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    /// Test-only hook: read back what was set. Not part of the port.
    pub fn peek(&self) -> Option<String> {
        self.state.lock().ok().and_then(|g| g.clone())
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
