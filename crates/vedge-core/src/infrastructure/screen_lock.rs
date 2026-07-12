//! OS screen-lock watcher adapters.
//!
//! Implement the [`ScreenLockWatcher`](crate::application::vault::ports::ScreenLockWatcher)
//! port. The real platform impl lives behind `#[cfg]`; [`platform_screen_lock_watcher`]
//! selects it, so shells stay platform-agnostic. Mirrors
//! [`infrastructure::biometric`](crate::infrastructure::biometric) minus the crypto
//! dependency — screen-lock detection derives no key, it only asks the OS a question.

use std::sync::Arc;

use crate::application::vault::ports::screen_lock::ScreenLockWatcher;

pub mod memory;

#[cfg(windows)]
pub mod windows;

#[cfg(not(windows))]
pub mod stub;

pub use memory::MemoryScreenLockWatcher;

#[cfg(windows)]
pub use windows::WindowsScreenLockWatcher;

#[cfg(not(windows))]
pub use stub::StubScreenLockWatcher;

/// The screen-lock watcher for the current platform.
///
/// Windows → a `WTSQuerySessionInformationW` poll; every other target → a stub
/// that reports `is_supported() == false` and never locks (macOS / Linux stay
/// password-only by design, exactly like the biometric stub).
#[must_use]
pub fn platform_screen_lock_watcher() -> Arc<dyn ScreenLockWatcher> {
    #[cfg(windows)]
    {
        Arc::new(WindowsScreenLockWatcher::new())
    }
    #[cfg(not(windows))]
    {
        Arc::new(StubScreenLockWatcher::new())
    }
}
