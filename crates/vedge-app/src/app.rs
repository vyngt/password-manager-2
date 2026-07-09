use crate::i18n::I18nContextProvider;

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
use vedge_ui::theme::{ThemeConfig, ThemeState, ThemeTokens, derive_tokens, inject_css_vars};

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

/// Flat, guaranteed-valid fallback token set derived from the base config colors.
///
/// Only reached if [`derive_tokens`] fails on the *seeded* [`default_theme_config`]
/// — effectively unreachable, since that literal is a valid theme. A `#[component]`
/// can't return `Result`, so rather than panic (which would blank the CSR app) we
/// degrade to flat base colors, keeping the app usable, and log the anomaly.
fn fallback_tokens(config: &ThemeConfig) -> ThemeTokens {
    let bg = config.background.clone();
    let fg = config.foreground.clone();
    let primary = config.primary.clone();
    ThemeTokens {
        color_background: bg.clone(),
        color_surface_1: bg.clone(),
        color_surface_2: bg.clone(),
        color_surface_3: bg.clone(),
        color_surface_4: bg.clone(),
        color_border: fg.clone(),
        color_border_strong: fg.clone(),

        color_text_primary: fg.clone(),
        color_text_secondary: fg.clone(),
        color_text_tertiary: fg,

        color_primary: primary.clone(),
        color_primary_hover: primary.clone(),
        color_primary_muted: primary.clone(),
        color_primary_foreground: bg.clone(),
        color_primary_text: primary.clone(),

        color_danger: "#DC2626".into(),
        color_danger_hover: "#DC2626".into(),
        color_danger_muted: "#DC2626".into(),
        color_danger_foreground: bg.clone(),
        color_danger_text: "#DC2626".into(),

        color_warning: "#D97706".into(),
        color_warning_hover: "#D97706".into(),
        color_warning_muted: "#D97706".into(),
        color_warning_foreground: bg.clone(),
        color_warning_text: "#D97706".into(),

        color_success: "#16A34A".into(),
        color_success_hover: "#16A34A".into(),
        color_success_muted: "#16A34A".into(),
        color_success_foreground: bg,
        color_success_text: "#16A34A".into(),

        color_focus_ring: primary,

        shadow_sm: "0 1px 2px rgba(0,0,0,0.05)".into(),
        shadow_md: "0 4px 6px rgba(0,0,0,0.1)".into(),
        shadow_lg: "0 10px 15px rgba(0,0,0,0.1)".into(),
        shadow_scrim: "rgba(0,0,0,0.5)".into(),
    }
}

#[component]
pub fn App() -> impl IntoView {
    let config = default_theme_config();
    let tokens = derive_tokens(&config).unwrap_or_else(|_| {
        web_sys::console::error_1(
            &"App: derive_tokens failed on the seeded default theme; using flat fallback tokens."
                .into(),
        );
        fallback_tokens(&config)
    });

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
