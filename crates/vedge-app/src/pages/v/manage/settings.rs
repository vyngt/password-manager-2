//! Settings — sectioned shell (Security | Appearance).
//!
//! The **Security** section (2.6) holds idle auto-lock, lock-on-blur, and the
//! clipboard clear delay, all bound to the app-global [`SecurityPrefsCtx`].
//! Editing a control updates the live signal, persists immediately via
//! `security_prefs::save`, and confirms with a Success toast. **Appearance** is
//! the theme manager (2.6.1).
//!
//! Sections use the design-system `Tabs`. The Security controls are gated behind
//! [`SecurityPrefsLoaded`] (a `Spinner` fallback) so they don't flash the default
//! prefs before the persisted values resolve.
//!
//! `Select` option lists are built inside reactive `{move || …}` closures (the
//! house pattern in `vault_filters`): the `t_string!` reads are then tracked —
//! no owner-less warning — and the labels relocalize on a language switch.

use crate::features::settings::biometric_setting::BiometricSetting;
use crate::features::settings::maintenance_panel::MaintenancePanel;
use crate::features::settings::security_prefs::{self, SecurityPrefsCtx, SecurityPrefsLoaded};
use crate::features::settings::theme_list::ThemeList;
use crate::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use std::sync::Arc;
use vedge_ui::components::Spinner;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::components::tabs::{Tab, Tabs, TabsVariant};
use vedge_ui::components::toggle::Toggle;
use vedge_ui::primitives::tokens::ToastVariant;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let i18n = use_i18n();
    let sec = expect_context::<SecurityPrefsCtx>();
    let loaded = expect_context::<SecurityPrefsLoaded>();
    let toast = use_toast();

    // Confirm each saved edit with a Success toast (replaces the old inline
    // "Saved" line that never reset). Reads are `untrack`ed so it's owner-safe.
    let show_saved = move || {
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        let msg = untrack(|| t_string!(i18n, settings.saved).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };

    let on_minutes = Callback::new(move |v: String| {
        let m = v.parse::<u32>().unwrap_or(0);
        sec.0.update(|p| p.auto_lock_minutes = m);
        security_prefs::save(&sec.0.get_untracked());
        show_saved();
    });
    let on_session_max = Callback::new(move |v: String| {
        let m = v.parse::<u32>().unwrap_or(480);
        sec.0.update(|p| p.session_max_minutes = m);
        security_prefs::save(&sec.0.get_untracked());
        show_saved();
    });
    let on_blur = Callback::new(move |v: bool| {
        sec.0.update(|p| p.lock_on_blur = v);
        security_prefs::save(&sec.0.get_untracked());
        show_saved();
    });
    let on_clipboard = Callback::new(move |v: String| {
        let s = v.parse::<u32>().unwrap_or(30);
        sec.0.update(|p| p.clipboard_clear_seconds = s);
        security_prefs::save(&sec.0.get_untracked());
        show_saved();
    });
    let on_breach = Callback::new(move |v: bool| {
        sec.0.update(|p| p.breach_check_enabled = v);
        security_prefs::save(&sec.0.get_untracked());
        show_saved();
    });

    // Security panel — gated behind `loaded` so the controls don't flash the
    // default prefs before the persisted values arrive.
    let security_panel = Arc::new(move || {
        view! {
            <Show
                when=move || loaded.0.get()
                fallback=|| {
                    view! {
                        <div class="flex justify-center py-10">
                            <Spinner />
                        </div>
                    }
                }
            >
                // ---- Auto-lock ------------------------------------
                <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.auto_lock)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.auto_lock_desc)}
                        </div>
                    </div>
                    <div class="w-44 shrink-0">
                        {move || {
                            let unit = t_string!(i18n, settings.minutes).to_owned();
                            let options = vec![
                                SelectItem::option(
                                    "0",
                                    t_string!(i18n, settings.auto_lock_off).to_owned(),
                                ),
                                SelectItem::option("1", format!("1 {unit}")),
                                SelectItem::option("5", format!("5 {unit}")),
                                SelectItem::option("15", format!("15 {unit}")),
                                SelectItem::option("30", format!("30 {unit}")),
                                SelectItem::option("60", format!("60 {unit}")),
                            ];
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        sec.0.get().auto_lock_minutes.to_string()
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, settings.auto_lock).to_owned()
                                    })
                                    on_change=on_minutes
                                />
                            }
                        }}
                    </div>
                </div>

                // ---- Max session (hard TTL, slice 4.5a) -----------
                <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.session_max)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.session_max_desc)}
                        </div>
                    </div>
                    <div class="w-44 shrink-0">
                        {move || {
                            let unit = t_string!(i18n, settings.hours).to_owned();
                            let options = vec![
                                SelectItem::option(
                                    "0",
                                    t_string!(i18n, settings.session_max_off).to_owned(),
                                ),
                                SelectItem::option("60", format!("1 {unit}")),
                                SelectItem::option("240", format!("4 {unit}")),
                                SelectItem::option("480", format!("8 {unit}")),
                                SelectItem::option("1440", format!("24 {unit}")),
                            ];
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        sec.0.get().session_max_minutes.to_string()
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, settings.session_max).to_owned()
                                    })
                                    on_change=on_session_max
                                />
                            }
                        }}
                    </div>
                </div>

                // ---- Lock on window blur --------------------------
                <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.lock_on_blur)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.lock_on_blur_desc)}
                        </div>
                    </div>
                    <div class="shrink-0 mt-0.5">
                        <Toggle
                            checked=Signal::derive(move || sec.0.get().lock_on_blur)
                            on_change=on_blur
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, settings.lock_on_blur).to_owned()
                            })
                        />
                    </div>
                </div>

                // ---- Clear clipboard ------------------------------
                <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.clipboard_clear)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.clipboard_clear_desc)}
                        </div>
                    </div>
                    <div class="w-44 shrink-0">
                        {move || {
                            let unit = t_string!(i18n, settings.seconds).to_owned();
                            let options = vec![
                                SelectItem::option("15", format!("15 {unit}")),
                                SelectItem::option("30", format!("30 {unit}")),
                                SelectItem::option("45", format!("45 {unit}")),
                                SelectItem::option("60", format!("60 {unit}")),
                            ];
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        sec.0.get().clipboard_clear_seconds.to_string()
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, settings.clipboard_clear).to_owned()
                                    })
                                    on_change=on_clipboard
                                />
                            }
                        }}
                    </div>
                </div>

                // ---- Breach detection (slice 4.4) -----------------
                <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.breach_check)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.breach_check_desc)}
                        </div>
                    </div>
                    <div class="shrink-0 mt-0.5">
                        <Toggle
                            checked=Signal::derive(move || sec.0.get().breach_check_enabled)
                            on_change=on_breach
                            aria_label=Signal::derive(move || {
                                t_string!(i18n, settings.breach_check).to_owned()
                            })
                        />
                    </div>
                </div>

                // ---- Biometric unlock (slice 2.8) -----------------
                <BiometricSetting />
            </Show>
        }
        .into_any()
    });

    let tabs = vec![
        Tab {
            id: "security".to_owned(),
            label: Box::new(move || view! { {move || t!(i18n, settings.security)} }.into_any()),
            panel: security_panel,
            disabled: false,
        },
        Tab {
            id: "appearance".to_owned(),
            label: Box::new(move || view! { {move || t!(i18n, settings.appearance)} }.into_any()),
            panel: Arc::new(|| view! { <ThemeList /> }.into_any()),
            disabled: false,
        },
        Tab {
            id: "maintenance".to_owned(),
            label: Box::new(move || view! { {move || t!(i18n, settings.maintenance)} }.into_any()),
            panel: Arc::new(|| view! { <MaintenancePanel /> }.into_any()),
            disabled: false,
        },
    ];

    view! {
        <div class="h-full overflow-y-auto p-6">
            <div class="max-w-2xl mx-auto">
                <Tabs tabs=tabs variant=TabsVariant::Underline />
            </div>
        </div>
    }
}
