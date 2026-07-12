//! Windows screen-lock watcher — polls the WTS session flags.
//!
//! [`WTSQuerySessionInformationW`] with `WTSSessionInfoEx` returns a `WTSINFOEX`
//! whose `WTSINFOEX_LEVEL1.SessionFlags` reports the console session's lock state
//! (`WTS_SESSIONSTATE_LOCK` / `_UNLOCK`). Two FFI calls (query + free) and one
//! union read — **no HWND, no WNDPROC, no message loop, no pointer smuggling**
//! (the event-driven `WTSRegisterSessionNotification` route was rejected for
//! exactly that cost; see the slice spec).
//!
//! ## Flag-semantics — confirmed on-device (slice 4.5b)
//!
//! The lock/unlock *sense* of `SessionFlags` is documented as possibly **inverted
//! on the console session** (reportedly since Windows 7) — an inversion would lock
//! the vault whenever the screen is *un*locked. So it was **smoked before wiring**
//! (acceptance criterion #1): on the target (2026-07-12) `SessionFlags` read **`1`
//! (`WTS_SESSIONSTATE_UNLOCK`) while unlocked** and **`0` (`WTS_SESSIONSTATE_LOCK`)
//! while locked** — the documented, non-inverted sense. So [`WindowsScreenLockWatcher::is_screen_locked`]
//! returns `Ok(flags == 0)`. A query failure returns `Err`, which the scheduler
//! fails open on (never locks). The raw flag is still logged on change for
//! diagnostics.
//!
//! **Verification note:** the lock/unlock transition cannot be exercised headless
//! (`WebDriver` cannot lock a workstation). It is validated by the manual on-device
//! smoke recorded in this slice's changelog — mirroring the Windows Hello
//! adapter's identical note. The rest of the stack is covered headless by
//! `MemoryScreenLockWatcher`.

use std::sync::atomic::{AtomicI64, Ordering};

use windows::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SESSION, WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW, WTSSessionInfoEx,
};
use windows::core::PWSTR;

use crate::application::vault::ports::screen_lock::ScreenLockWatcher;
use crate::domain::vault::errors::VaultError;

/// Sentinel for "no flag logged yet" in the dedup cache. Real `SessionFlags` are
/// small `LONG`s (`0`/`1`, or `-1` for unknown), so `i64::MIN` can't collide.
const FLAG_UNLOGGED: i64 = i64::MIN;

/// The `SessionFlags` value meaning "locked" — `WTS_SESSIONSTATE_LOCK`. Confirmed
/// on-device (2026-07-12) as the non-inverted sense (unlocked reads `1` =
/// `WTS_SESSIONSTATE_UNLOCK`); see this module's header.
const SESSION_FLAGS_LOCKED: i32 = 0;

pub struct WindowsScreenLockWatcher {
    /// Last `SessionFlags` value we logged, so the diagnostic logs each *change*
    /// once instead of on every 5 s tick. `i64` holds the `i32` flag plus the
    /// [`FLAG_UNLOGGED`] sentinel.
    last_logged: AtomicI64,
}

impl WindowsScreenLockWatcher {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_logged: AtomicI64::new(FLAG_UNLOGGED),
        }
    }

    /// Log the raw `SessionFlags` at info level, but only when it changes from the
    /// last logged value — quiet on every unchanged tick.
    fn log_flag_on_change(&self, flags: i32) {
        let cur = i64::from(flags);
        if self.last_logged.swap(cur, Ordering::Relaxed) != cur {
            tracing::info!(
                session_flags = flags,
                locked = flags == SESSION_FLAGS_LOCKED,
                "screen-lock: WTS SessionFlags changed"
            );
        }
    }
}

impl Default for WindowsScreenLockWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII wrapper freeing a `WTSQuerySessionInformationW` buffer via `WTSFreeMemory`.
/// A leak here would leak once **per poll tick**, so freeing is not optional.
struct WtsBuffer(PWSTR);

#[allow(unsafe_code)]
impl Drop for WtsBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` was allocated by `WTSQuerySessionInformationW` and
            // is freed exactly once — here, on drop. Null is guarded just above.
            unsafe { WTSFreeMemory(self.0.0.cast::<core::ffi::c_void>()) };
        }
    }
}

/// Query the console session's raw `SessionFlags`. Two FFI calls + one union
/// read; the buffer is freed by [`WtsBuffer`]'s `Drop` on every path.
//
// `cast_ptr_alignment`: the buffer typed `*mut u16` (`PWSTR`) is really a
// WTS-allocated `WTSINFOEXW`, which the OS returns correctly aligned — the u16
// element type is an artifact of the shared `LPWSTR* ppBuffer` out-param signature.
#[allow(unsafe_code, clippy::cast_ptr_alignment)]
fn query_session_flags() -> Result<i32, VaultError> {
    let mut buffer = PWSTR::null();
    let mut bytes: u32 = 0;

    // SAFETY: the out-params are valid for the duration of the call. On success
    // `buffer` receives a WTS-allocated `WTSINFOEXW`, wrapped immediately below in
    // `WtsBuffer` for freeing. `None` is the null server handle =
    // `WTS_CURRENT_SERVER_HANDLE` (the local server).
    unsafe {
        WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &raw mut buffer,
            &raw mut bytes,
        )
    }
    .map_err(|_| VaultError::ScreenLockUnavailable)?;

    let guard = WtsBuffer(buffer);
    if guard.0.is_null() {
        return Err(VaultError::ScreenLockUnavailable);
    }
    // Guard the reported size before treating the buffer as a `WTSINFOEXW`: WTS
    // writes the byte count it filled, and dereferencing a short/legacy buffer as
    // the full struct would be an out-of-bounds read. Defensive — the info class
    // fails cleanly when unsupported, but don't rely on that alone.
    if usize::try_from(bytes).unwrap_or(0) < size_of::<WTSINFOEXW>() {
        return Err(VaultError::ScreenLockUnavailable);
    }

    // SAFETY: `guard.0` is the non-null `WTSINFOEXW` from the successful query, and
    // `bytes` is at least its size (checked above).
    let info: &WTSINFOEXW = unsafe { &*guard.0.0.cast::<WTSINFOEXW>() };
    // Check the union discriminant BEFORE reading it: only `Level == 1` means the
    // `WTSInfoExLevel1` arm is the active one, so we never read an inactive variant.
    if info.Level != 1 {
        return Err(VaultError::ScreenLockUnavailable);
    }
    // SAFETY: `Level == 1` guarantees the `WTSInfoExLevel1` union arm is active.
    let flags = unsafe { info.Data.WTSInfoExLevel1.SessionFlags };
    Ok(flags)
}

impl ScreenLockWatcher for WindowsScreenLockWatcher {
    fn is_supported(&self) -> bool {
        true
    }

    fn is_screen_locked(&self) -> Result<bool, VaultError> {
        let flags = query_session_flags()?;
        self.log_flag_on_change(flags);
        // Confirmed on-device (see header): locked ⇔ `SessionFlags == 0`
        // (`WTS_SESSIONSTATE_LOCK`); unlocked reads `1`. A query failure already
        // returned `Err` above, which the scheduler fails open on.
        Ok(flags == SESSION_FLAGS_LOCKED)
    }
}
