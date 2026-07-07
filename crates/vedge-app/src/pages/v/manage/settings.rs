//! Settings — sectioned shell (Security | Appearance).
//!
//! This slice (2.6) fills the **Security** section: idle auto-lock, lock-on-blur,
//! and the clipboard clear delay, all bound to the app-global
//! [`SecurityPrefsCtx`]. Editing a control updates the live signal and persists
//! immediately via `security_prefs::save`. **Appearance** is a placeholder that
//! 2.6.1 (Theme Manager) fills without reworking the shell.
//!
//! `Select` option lists are built inside reactive `{move || …}` closures (the
//! house pattern in `vault_filters`): the `t_string!` reads are then tracked —
//! no owner-less warning — and the labels relocalize on a language switch.

use crate::features::settings::security_prefs::{self, SecurityPrefsCtx};
use crate::features::settings::theme_list::ThemeList;
use crate::i18n::*;
use icondata as i;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::components::toggle::Toggle;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Security,
    Appearance,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let i18n = use_i18n();
    let sec = expect_context::<SecurityPrefsCtx>();
    let tab = RwSignal::new(Tab::Security);
    // Flipped true after any successful edit → shows the "Saved" line.
    let saved = RwSignal::new(false);

    let on_minutes = Callback::new(move |v: String| {
        let m = v.parse::<u32>().unwrap_or(0);
        sec.0.update(|p| p.auto_lock_minutes = m);
        security_prefs::save(sec.0.get_untracked());
        saved.set(true);
    });
    let on_blur = Callback::new(move |v: bool| {
        sec.0.update(|p| p.lock_on_blur = v);
        security_prefs::save(sec.0.get_untracked());
        saved.set(true);
    });
    let on_clipboard = Callback::new(move |v: String| {
        let s = v.parse::<u32>().unwrap_or(30);
        sec.0.update(|p| p.clipboard_clear_seconds = s);
        security_prefs::save(sec.0.get_untracked());
        saved.set(true);
    });

    view! {
        <div class="h-full overflow-y-auto p-6">
            <div class="max-w-2xl mx-auto">
                // ---- Section tabs -------------------------------------------------
                <div class="flex gap-5 text-sm border-b border-border mb-5">
                    <button
                        type="button"
                        class="pb-2.5 border-b-2"
                        class=("border-primary", move || tab.get() == Tab::Security)
                        class=("text-text-primary", move || tab.get() == Tab::Security)
                        class=("border-transparent", move || tab.get() != Tab::Security)
                        class=("text-text-secondary", move || tab.get() != Tab::Security)
                        on:click=move |_| tab.set(Tab::Security)
                    >
                        {move || t!(i18n, settings.security)}
                    </button>
                    <button
                        type="button"
                        class="pb-2.5 border-b-2"
                        class=("border-primary", move || tab.get() == Tab::Appearance)
                        class=("text-text-primary", move || tab.get() == Tab::Appearance)
                        class=("border-transparent", move || tab.get() != Tab::Appearance)
                        class=("text-text-secondary", move || tab.get() != Tab::Appearance)
                        on:click=move |_| tab.set(Tab::Appearance)
                    >
                        {move || t!(i18n, settings.appearance)}
                    </button>
                </div>

                {move || match tab.get() {
                    Tab::Security => {
                        Either::Left(
                            view! {
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
                                            let unit = t_string!(i18n, settings.minutes).to_string();
                                            let options = vec![
                                                SelectItem::option(
                                                    "0",
                                                    t_string!(i18n, settings.auto_lock_off).to_string(),
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
                                                    on_change=on_minutes
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
                                                t_string!(i18n, settings.lock_on_blur).to_string()
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
                                            let unit = t_string!(i18n, settings.seconds).to_string();
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
                                                    on_change=on_clipboard
                                                />
                                            }
                                        }}
                                    </div>
                                </div>

                                <Show when=move || saved.get()>
                                    <div class="inline-flex items-center gap-1.5 text-xs text-success-text mt-4">
                                        <Icon icon=i::FaCircleCheckSolid />
                                        {move || t!(i18n, settings.saved)}
                                    </div>
                                </Show>
                            },
                        )
                    }
                    Tab::Appearance => Either::Right(view! { <ThemeList /> }),
                }}
            </div>
        </div>
    }
}
