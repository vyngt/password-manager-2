//! App-level active-vault context.
//!
//! Provided once at the app root (`app.rs`). Onboarding + unlock **set**
//! `path` on success; entry screens (slices 1.4/1.5) **read** it via
//! `expect_context::<ActiveVault>()` to know which vault to operate on.

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct ActiveVault {
    /// `Some(path)` once a vault is unlocked/created; `None` otherwise.
    pub path: RwSignal<Option<String>>,
}

impl ActiveVault {
    #[must_use]
    pub fn new() -> Self {
        Self {
            path: RwSignal::new(None),
        }
    }
}

impl Default for ActiveVault {
    fn default() -> Self {
        Self::new()
    }
}

/// The show-once new Secret Key from a re-key that rotated it (slice 5.8).
///
/// Re-key LOCKS the vault on success, so the key can't be shown on the settings dialog — the
/// auto-lock watcher ejects it to the launch screen within ~1 s. Instead the re-key dialog stashes
/// the `A3-…` display here (a context provided **above the router**, so it survives the `/v`
/// unmount) and navigates to the launch screen, which shows it in a dismissable panel where there
/// is no auto-lock. Cleared when the user dismisses it.
#[derive(Clone, Copy)]
pub struct NewSecretKit {
    pub display: RwSignal<Option<String>>,
}

impl NewSecretKit {
    #[must_use]
    pub fn new() -> Self {
        Self {
            display: RwSignal::new(None),
        }
    }
}

impl Default for NewSecretKit {
    fn default() -> Self {
        Self::new()
    }
}
