//! `SecretMem<T>` — a heap-pinned, page-sized, `mlock()`-protected wrapper
//! around long-lived key material.
//!
//! The KEK needs to live for the entire vault session and must not silently
//! end up in a swap file on disk. `mlock()` (via the cross-platform `region`
//! crate) pins the containing page to physical RAM. Drop releases the lock
//! and zeroizes the bytes before the allocator reclaims them.
//!
//! This is *not* a protection against kernel-level dumps or cold-boot attacks
//! — those are out of scope for userspace. It specifically ensures key
//! material does not appear in `pagefile.sys` / `/swap`.
//!
//! ## Why page-aligned + page-sized?
//!
//! `region::lock` works at OS page granularity. If two `SecretMem<[u8; 32]>`
//! land on the same 4 KiB page (small allocations co-locate), the second
//! `VirtualUnlock` / `munlock` on drop reports the page "already unlocked"
//! — a panic in `LockGuard::Drop`. We work around it by forcing every
//! `SecretMem` into its own page via `#[repr(align(4096))]` — the struct
//! occupies exactly one page, and its Box gets a page-aligned allocation.

use std::pin::Pin;

use region::LockGuard;
use zeroize::Zeroize;

use crate::domain::vault::errors::VaultError;

// 4 KiB — the page size on every platform we target (x86_64, aarch64-macOS).
// arm64-Linux with 64 KiB pages would still satisfy "lock a 4 KiB subrange"
// but would waste more per instance; the lock call handles it regardless.
const PAGE_BYTES: usize = 4096;

#[repr(C, align(4096))]
#[derive(Clone, Copy)]
struct PageAligned<T: Copy> {
    value: T,
}

impl<T: Copy + Default> Default for PageAligned<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
        }
    }
}

impl<T: Copy + Zeroize> Zeroize for PageAligned<T> {
    fn zeroize(&mut self) {
        self.value.zeroize();
    }
}

pub struct SecretMem<T: Copy + Default + Zeroize + Unpin> {
    // Declaration order = field drop order. `_lock_guard` MUST be declared
    // first so its `region::unlock` runs while `inner`'s memory is still
    // mapped. If `inner` dropped first the Box would free the pages, then
    // the LockGuard's unlock would call into freed memory and the OS would
    // report "segment is already unlocked" (observed on Windows).
    _lock_guard: Option<LockGuard>,
    // Pinned + page-aligned so the pointer we hand to `region::lock` targets
    // a page this allocation fully owns.
    inner: Pin<Box<PageAligned<T>>>,
}

impl<T: Copy + Default + Zeroize + Unpin> SecretMem<T> {
    /// Pin `value` on the heap (inside a page-sized allocation) and lock its
    /// pages in RAM.
    ///
    /// Returns [`VaultError::MlockFailed`] if the OS refused the lock
    /// (insufficient privileges, rlimit, etc.). Callers treat this as fatal —
    /// the unlock flow cannot proceed without an mlock'd KEK.
    pub fn new(value: T) -> Result<Self, VaultError> {
        let inner = Box::pin(PageAligned { value });
        let ptr = (&raw const *inner).cast::<u8>();
        // Size = one page; `align(4096)` ensures the struct occupies a full
        // page already. If `T` ever grew beyond a page we'd take several
        // pages — fine, `region::lock` handles multi-page ranges.
        let size = std::cmp::max(std::mem::size_of::<PageAligned<T>>(), PAGE_BYTES);
        let guard = Some(region::lock(ptr, size).map_err(|_| VaultError::MlockFailed)?);
        Ok(Self {
            _lock_guard: guard,
            inner,
        })
    }

    /// Borrow the protected value.
    #[must_use]
    pub fn expose(&self) -> &T {
        &self.inner.value
    }

    /// Overwrite the protected bytes in-place.
    pub fn zeroize_in_place(&mut self) {
        self.inner.value.zeroize();
    }
}

impl<T: Copy + Default + Zeroize + Unpin> Drop for SecretMem<T> {
    fn drop(&mut self) {
        // Zeroize first; then field drop order handles unlock-before-free.
        self.inner.value.zeroize();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn round_trip_expose() {
        let sm: SecretMem<[u8; 32]> = SecretMem::new([7u8; 32]).unwrap();
        assert_eq!(sm.expose(), &[7u8; 32]);
    }

    #[test]
    fn zeroize_in_place_overwrites() {
        let mut sm: SecretMem<[u8; 32]> = SecretMem::new([0xAA; 32]).unwrap();
        sm.zeroize_in_place();
        assert_eq!(sm.expose(), &[0u8; 32]);
    }

    #[test]
    fn drop_zeroizes_before_release() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);

        #[derive(Clone, Copy, Default)]
        struct Probe([u8; 32]);

        impl Zeroize for Probe {
            fn zeroize(&mut self) {
                CALLS.fetch_add(1, Ordering::SeqCst);
                self.0.zeroize();
            }
        }

        let before = CALLS.load(Ordering::SeqCst);
        {
            let _sm: SecretMem<Probe> = SecretMem::new(Probe([9u8; 32])).unwrap();
        }
        let after = CALLS.load(Ordering::SeqCst);
        assert!(after > before, "Drop did not call zeroize");
    }

    #[test]
    fn many_concurrent_secretmems_do_not_conflict() {
        // Smoke test for the page-alignment + drop-order fix: allocate and
        // drop multiple SecretMem instances without triggering the
        // "segment already unlocked" path.
        let mut kept = Vec::new();
        for i in 0u8..16u8 {
            kept.push(SecretMem::<[u8; 32]>::new([i; 32]).unwrap());
        }
        // Drop in a shuffled-ish order.
        kept.reverse();
        drop(kept);
    }
}
