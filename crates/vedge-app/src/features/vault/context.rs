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
