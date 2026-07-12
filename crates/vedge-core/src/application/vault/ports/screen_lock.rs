use crate::domain::vault::errors::VaultError;

/// Watches the OS session-lock state so the vault can lock itself when the
/// **workstation** locks (Windows lock screen, lock-on-sleep).
///
/// **Query-shaped, deliberately.** The poll is driven by the shell's periodic
/// task (slice 4.5a's scheduler), so this port stays a pure, synchronous,
/// trivially-fakeable *question* rather than a callback registration. A
/// callback-registering port (`WTSRegisterSessionNotification` + a `WNDPROC`)
/// would force the FFI's threading model — an HWND, a message loop, a raw
/// pointer round-trip through OS-owned memory — into the trait and into every
/// test double. See the slice spec for why the poll was chosen over the
/// event-driven route.
///
/// Synchronous — like [`BiometricAuthenticator`](super::biometric::BiometricAuthenticator)
/// and [`KeychainProvider`](super::keychain::KeychainProvider), the underlying OS
/// call is blocking (and fast — a single `WTSQuerySessionInformationW`), so there
/// is nothing to `await`.
///
/// **Error-mapping contract (infrastructure is responsible):** any FFI failure
/// maps to [`VaultError::ScreenLockUnavailable`]. A failed query must **never**
/// lock the vault — the caller fails *open* (logs and does not lock). The
/// asymmetry is deliberate: a false positive locks a working user out of their
/// vault mid-task, while a false negative only costs a few seconds behind an
/// already-locked screen.
pub trait ScreenLockWatcher: Send + Sync {
    /// Whether OS screen-lock detection is implemented on this platform. `false`
    /// on the stub (non-Windows) — callers treat the workstation as never locked
    /// and nothing regresses off Windows.
    fn is_supported(&self) -> bool;

    /// Whether the workstation is currently locked. Never prompts and never
    /// blocks on user input. Returns [`VaultError::ScreenLockUnavailable`] on an
    /// FFI failure — the caller must fail *open* (not lock).
    fn is_screen_locked(&self) -> Result<bool, VaultError>;
}
