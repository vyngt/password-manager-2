//! Shared vault-UI state, provided at the `/v` layout.
//!
//! Lifts the pieces the command palette must drive out of `VaultPage` so they
//! can be read/written from anywhere under `/v`: the **selection**
//! (`selected_id`), the **create-form toggle** (`show_create`), the palette's
//! own open flag (`palette_open`), and an **edit-request** trigger the open
//! detail panel observes. `VaultPage` consumes the same context, so mouse
//! behavior is unchanged. This is the surface later slices (2.4 favorites, 2.5
//! folders) add commands to. Mirrors the [`ActiveVault`](super::context) shape:
//! a `Copy` struct of `RwSignal`s provided via `provide_context` and read via
//! `expect_context::<VaultUiState>()`.

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct VaultUiState {
    /// Currently selected entry id (drives the detail panel).
    pub selected_id: RwSignal<Option<String>>,
    /// Whether the inline create-entry form is shown.
    pub show_create: RwSignal<bool>,
    /// Whether the command palette modal is open.
    pub palette_open: RwSignal<bool>,
    /// Bumped to ask the open detail panel to enter edit mode (it observes the
    /// change; the value itself is a monotonic ticket, not meaningful).
    pub edit_request: RwSignal<u32>,
}

impl VaultUiState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            selected_id: RwSignal::new(None),
            show_create: RwSignal::new(false),
            palette_open: RwSignal::new(false),
            edit_request: RwSignal::new(0),
        }
    }
}

impl Default for VaultUiState {
    fn default() -> Self {
        Self::new()
    }
}
