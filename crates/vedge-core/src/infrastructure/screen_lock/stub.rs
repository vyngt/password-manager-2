use crate::application::vault::ports::screen_lock::ScreenLockWatcher;
use crate::domain::vault::errors::VaultError;

/// Screen-lock watcher for platforms without a native implementation.
///
/// macOS and Linux stay password-only by design (mirroring the biometric stub).
/// Reports unsupported and never locks, so nothing regresses off Windows.
pub struct StubScreenLockWatcher;

impl StubScreenLockWatcher {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for StubScreenLockWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenLockWatcher for StubScreenLockWatcher {
    fn is_supported(&self) -> bool {
        false
    }

    fn is_screen_locked(&self) -> Result<bool, VaultError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_watcher_never_locks() {
        let w = StubScreenLockWatcher::new();
        assert!(!w.is_supported());
        assert!(matches!(w.is_screen_locked(), Ok(false)));
    }
}
