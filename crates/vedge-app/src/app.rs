use crate::i18n::*;

use leptos::prelude::*;

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

    view! {
        <I18nContextProvider>
            <div class="h-full flex flex-col bg-background text-text-primary">
                <ToastProvider>
                    <WindowPanel />
                    <main class="h-[calc(100%-48px)] overflow-y-auto app-scrollbar">
                        <AppRoutes />
                    </main>
                </ToastProvider>
            </div>
        </I18nContextProvider>
    }
}
