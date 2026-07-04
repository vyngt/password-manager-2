use crate::i18n::*;

use leptos::prelude::*;

use crate::features::vault::context::ActiveVault;
use crate::features::window_panel::WindowPanel;
use crate::routes::AppRoutes;
use vedge_ui::components::feedback::toast::provider::ToastProvider;
use vedge_ui::theme::{ThemeConfig, ThemeState, derive_tokens, inject_css_vars};

/// Default light theme configuration.
fn default_theme_config() -> ThemeConfig {
    ThemeConfig {
        background: "#FFFFFF".into(),
        foreground: "#111827".into(),
        primary: "#2563EB".into(),
        danger: None,
        warning: None,
        success: None,
    }
}

#[component]
pub fn App() -> impl IntoView {
    let config = default_theme_config();
    let tokens = derive_tokens(&config).expect("default theme must be valid");

    // Inject CSS vars on startup
    inject_css_vars(&tokens);

    let theme = ThemeState::new(
        tokens,
        |_config| {
            // TODO: wire to Tauri invoke("save_theme", ...) when backend is ready
        },
        || {
            // TODO: wire to Tauri invoke("apply_active_theme") when backend is ready
        },
    );

    provide_context(theme);
    provide_context(ActiveVault::new());

    view! {
        <I18nContextProvider>
            <ToastProvider>
                <WindowPanel />
                <main class="h-[calc(100%-var(--titlebar-height))] overflow-y-auto app-scrollbar">
                    <AppRoutes />
                </main>
            </ToastProvider>
        </I18nContextProvider>
    }
}
