use crate::i18n::I18nContextProvider;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::features::generator::history::GeneratedHistoryCtx;
use crate::features::health::context::HealthReportCtx;
use crate::features::settings::generator_prefs::{self, GeneratorPrefs, GeneratorPrefsCtx};
use crate::features::settings::security_prefs::{
    self, SecurityPrefs, SecurityPrefsCtx, SecurityPrefsLoaded,
};
use crate::features::settings::theme_util::theme_config_from_dto;
use crate::features::vault::context::{ActiveVault, NewSecretKit};
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

/// Install debug-only JS test hooks on `window` for the e2e harness.
///
/// Currently just `window.__vedge_test_dialog(on: bool)`, which toggles the
/// renderer-side in-dialog counter so the lock-on-blur **negative** e2e assertion
/// can simulate an open native dialog (a native OS dialog can't be driven in
/// `WebDriver`). Absent from release builds (`cfg(debug_assertions)`). The `Closure`
/// is intentionally `forget`-leaked — it must outlive this call, and there is
/// exactly one for the app's lifetime.
#[cfg(debug_assertions)]
fn install_test_hooks() {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let Some(win) = web_sys::window() else {
        return;
    };
    let cb = Closure::<dyn Fn(bool)>::new(|on: bool| {
        crate::api::dialog::test_set_dialog_in_progress(on);
    });
    let _ = js_sys::Reflect::set(
        &win,
        &wasm_bindgen::JsValue::from_str("__vedge_test_dialog"),
        cb.as_ref().unchecked_ref(),
    );
    cb.forget();
}

#[component]
pub fn App() -> impl IntoView {
    #[cfg(debug_assertions)]
    install_test_hooks();

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
    let active = ActiveVault::new();
    provide_context(active);
    // The show-once new Secret Key from a re-key (5.8): provided above the router so it survives the
    // `/v` unmount that fires when re-key locks the vault, then shown on the launch screen.
    provide_context(NewSecretKit::new());

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

    // App-global generator preset (last-used charset config), shared by the
    // inline quick-generate button, the expand-to-tune popover, and the
    // standalone `/v/generator` panel. Defaults are a valid starting preset, so
    // there's no load gate (nothing to hide before the persisted value arrives).
    let gen_prefs = GeneratorPrefsCtx(RwSignal::new(GeneratorPrefs::default()));
    provide_context(gen_prefs);
    Effect::new(move |_| {
        spawn_local(async move {
            gen_prefs.0.set(generator_prefs::load().await);
        });
    });

    // Session-only history of panel-generated secrets (slice 3.5): held in
    // `Zeroizing`, capped, never persisted. Provided here in the durable `App`
    // body (above the router) so it survives the `/v` unmount that fires on lock.
    let history = GeneratedHistoryCtx::new();
    provide_context(history);
    // Last password-health scan (slice 4.3b): held here (above the router) so it
    // survives navigation across `/v/*`, and — like the generator history — it is
    // sensitive derived data, so it is wiped on lock by the same Effect below.
    let health = HealthReportCtx::new();
    provide_context(health);
    // Wipe on lock. All three lock paths (idle auto-lock, sidebar Lock, palette
    // Lock) drive `active.path` Some→None; this prev-guarded Effect is the single
    // choke point. Guarded so the initial mount and unlock (None/Some(false)→…)
    // don't fire — only a true→false transition wipes. Evicted items zeroize on drop.
    Effect::new(move |was_unlocked: Option<bool>| {
        let unlocked = active.path.with(Option::is_some);
        if was_unlocked == Some(true) && !unlocked {
            history.clear();
            health.clear();
        }
        unlocked
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
