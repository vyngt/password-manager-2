use std::sync::Arc;

use leptos::prelude::*;

use super::derive::derive_tokens;
use super::types::{ThemeConfig, ThemeTokens, ThemeValidation};
use super::validate::validate_tokens;

/// Reactive theme state, provided via Leptos context.
///
/// Follows the same pattern as `ToastState`: a `Clone`-able struct holding
/// `RwSignal`s, registered with `provide_context` and retrieved with
/// `expect_context`.
///
/// The application layer constructs this with its own `commit_fn` and
/// `revert_fn` closures (which call Tauri commands). The UI library
/// only defines the shape.
#[derive(Clone)]
pub struct ThemeState {
    saved_tokens: RwSignal<ThemeTokens>,
    active_tokens: RwSignal<ThemeTokens>,
    commit_fn: Arc<dyn Fn(ThemeConfig) + Send + Sync>,
    revert_fn: Arc<dyn Fn() + Send + Sync>,
}

impl ThemeState {
    pub fn new(
        initial_tokens: ThemeTokens,
        commit_fn: impl Fn(ThemeConfig) + Send + Sync + 'static,
        revert_fn: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            saved_tokens: RwSignal::new(initial_tokens.clone()),
            active_tokens: RwSignal::new(initial_tokens),
            commit_fn: Arc::new(commit_fn),
            revert_fn: Arc::new(revert_fn),
        }
    }

    /// The currently active token set (reactive signal).
    /// Components should read this to stay in sync with theme changes.
    pub fn tokens(&self) -> RwSignal<ThemeTokens> {
        self.active_tokens
    }

    /// Validate a theme configuration against WCAG contrast requirements.
    pub fn validate(
        &self,
        config: &ThemeConfig,
    ) -> Result<ThemeValidation, super::color_space::ColorError> {
        let tokens = derive_tokens(config)?;
        validate_tokens(&tokens)
    }

    /// Apply a visual preview without persisting. Updates the active tokens
    /// signal and injects CSS variables on `:root`.
    pub fn preview(&self, config: &ThemeConfig) {
        if let Ok(tokens) = derive_tokens(config) {
            super::inject::inject_css_vars(&tokens);
            self.active_tokens.set(tokens);
        }
    }

    /// Persist the theme: derive, inject, update both signals, call the
    /// application's commit callback.
    pub fn commit(&self, config: &ThemeConfig) {
        if let Ok(tokens) = derive_tokens(config) {
            super::inject::inject_css_vars(&tokens);
            self.active_tokens.set(tokens.clone());
            self.saved_tokens.set(tokens);
            (self.commit_fn)(config.clone());
        }
    }

    /// Revert to the last saved theme: restore active tokens from saved,
    /// re-inject, and call the application's revert callback.
    pub fn revert(&self) {
        let saved = self.saved_tokens.get_untracked();
        #[cfg(target_arch = "wasm32")]
        super::inject::inject_css_vars(&saved);
        self.active_tokens.set(saved);
        (self.revert_fn)();
    }
}

/// Retrieve the `ThemeState` from Leptos context.
///
/// Panics if no `ThemeState` has been provided via `provide_context`.
pub fn use_theme() -> ThemeState {
    expect_context::<ThemeState>()
}
