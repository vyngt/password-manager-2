//! Biometric authenticator adapters (Windows Hello / Touch ID).
//!
//! Implement the [`BiometricAuthenticator`](crate::application::vault::ports::BiometricAuthenticator)
//! port. The real platform impl lives behind `#[cfg]`; [`platform_authenticator`]
//! selects it, so shells stay platform-agnostic.

use std::sync::Arc;

use crate::application::vault::ports::biometric::BiometricAuthenticator;
use crate::application::vault::ports::crypto::CryptoProvider;

pub mod memory;

#[cfg(windows)]
pub mod windows;

#[cfg(not(windows))]
pub mod stub;

pub use memory::MemoryBiometricAuthenticator;

#[cfg(windows)]
pub use windows::WindowsHelloAuthenticator;

#[cfg(not(windows))]
pub use stub::StubBiometricAuthenticator;

/// The biometric authenticator for the current platform.
///
/// Windows → Windows Hello (reuses `crypto` to wrap the KEK under the Hello-derived
/// key); every other target → a stub that reports `is_available() == false` (macOS
/// Touch ID lands in a future follow-up; Linux is password-only by design).
#[must_use]
// Windows consumes `crypto` (`WindowsHelloAuthenticator::new(crypto)`); the
// non-Windows arms only drop it, so clippy flags a by-value arg there. The
// signature must stay by-value for the Windows path — allow it off-Windows.
#[cfg_attr(not(windows), allow(clippy::needless_pass_by_value))]
pub fn platform_authenticator(crypto: Arc<dyn CryptoProvider>) -> Arc<dyn BiometricAuthenticator> {
    #[cfg(windows)]
    {
        Arc::new(WindowsHelloAuthenticator::new(crypto))
    }
    #[cfg(not(windows))]
    {
        let _ = crypto;
        Arc::new(StubBiometricAuthenticator::new())
    }
}
