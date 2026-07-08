use crate::i18n::*;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::features::settings::security_prefs::{
    self, SecurityPrefs, SecurityPrefsCtx, SecurityPrefsLoaded,
};
use crate::features::settings::theme_util::theme_config_from_dto;
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

    provide_context(theme.clone());
    provide_context(ActiveVault::new());

    // Boot into the *saved* active theme (falls back to the seeded light config
    // on error). `commit` re-derives, injects the `--color-*` cascade, and syncs
    // the active-tokens signal. Active-theme persistence is id-based via
    // `set_active_theme`, so the no-op `commit_fn` stub above is fine.
    Effect::new(move |_| {
        // No tracked reads → runs once after mount.
        let theme = theme.clone();
        spawn_local(async move {
            if let Ok(dto) = api::settings::get_active_theme().await {
                theme.commit(&theme_config_from_dto(&dto));
            }
        });
    });

    // App-global security prefs (idle auto-lock, lock-on-blur, clipboard clear
    // delay). Loaded once from `app_settings` and provided as a live context the
    // AutoLock hook, the Settings section, and both clipboard copy sites read.
    let security = SecurityPrefsCtx(RwSignal::new(SecurityPrefs::default()));
    let security_loaded = SecurityPrefsLoaded(RwSignal::new(false));
    provide_context(security);
    provide_context(security_loaded);
    Effect::new(move |_| {
        // No tracked reads → runs exactly once after mount.
        spawn_local(async move {
            security.0.set(security_prefs::load().await);
            security_loaded.0.set(true);
        });
    });

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
