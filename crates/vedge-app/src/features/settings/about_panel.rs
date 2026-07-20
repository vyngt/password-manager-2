//! About panel (slice PG.3) — the Settings → About tab.
//!
//! Carries four things in one surface: the app **version**
//! (`app.package_info().version`, fetched once on mount), a **manual update
//! check** (opt-in auto, never auto-install), `VEdge`'s **MIT** licence, and the
//! **third-party attribution** notices.
//!
//! 🔴 The update check is a **Rust command** (`api::about::check_for_update` →
//! `vedge-tauri/src/commands/update.rs`), so the frontend makes no HTTP request:
//! `capabilities/default.json` and the CSP's `connect-src` are byte-identical to
//! PG.2b. The attribution list is `include_str!`'d + parsed + rendered
//! **natively** (no `inner_html`), for the same reason.

use std::collections::{BTreeMap, BTreeSet};

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use vedge_ipc::{UpdateCheckDto, UpdateStatus};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::components::toggle::Toggle;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::features::settings::security_prefs::{self, SecurityPrefsCtx, SecurityPrefsLoaded};
use crate::i18n::{t, t_string, use_i18n};

/// The last update-check result, shared app-wide. Written by the manual check
/// here and by the quiet launch auto-check (`app.rs`); read by this panel so a
/// background result is already visible when the user opens the tab.
#[derive(Clone, Copy)]
pub struct UpdateStatusCtx(pub RwSignal<Option<UpdateCheckDto>>);

#[component]
pub fn AboutPanel() -> impl IntoView {
    let i18n = use_i18n();
    let toast = use_toast();
    let sec = expect_context::<SecurityPrefsCtx>();
    let loaded = expect_context::<SecurityPrefsLoaded>();
    let update_status = expect_context::<UpdateStatusCtx>();

    // Version — read once on mount (cheap, no network; works offline).
    let version = RwSignal::new(String::new());
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(v) = api::about::app_version().await {
                version.set(v);
            }
        });
    });

    // Manual "Check for updates" — always available (opt-in governs only the
    // background check). Writes the shared context, including `Unknown` on
    // failure so the inline "couldn't check" message shows (④).
    let checking = RwSignal::new(false);
    let check_now = move || {
        if checking.get_untracked() {
            return;
        }
        checking.set(true);
        spawn_local(async move {
            let result = api::about::check_for_update().await;
            checking.set(false);
            let dto = result.unwrap_or_else(|_| UpdateCheckDto {
                current: version.get_untracked(),
                latest: None,
                status: UpdateStatus::Unknown,
                release_url: None,
            });
            update_status.0.set(Some(dto));
        });
    };

    // Auto-check toggle — mirrors the breach-check toggle. Gated behind `loaded`
    // so a toggle before the persisted prefs arrive can't save over them.
    let saved_toast = move || {
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        let msg = untrack(|| t_string!(i18n, settings.saved).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };
    let on_auto = Callback::new(move |v: bool| {
        sec.0.update(|p| p.update_check_enabled = v);
        security_prefs::save(&sec.0.get_untracked());
        saved_toast();
    });

    // Third-party attribution — parsed once from the committed JSON, merged by
    // SPDX id. Static data, so no signal.
    let attributions = merged_attributions();

    view! {
        <div class="pt-2">
            // ---- Version --------------------------------------------------
            <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
                <div>
                    <div class="text-sm font-medium text-text-primary">
                        {move || t!(i18n, settings.about_version)}
                    </div>
                    <div class="text-xs text-text-secondary mt-0.5">
                        {move || t!(i18n, settings.about_version_desc)}
                    </div>
                </div>
                <div
                    class="shrink-0 mt-0.5 text-sm text-text-primary font-jetbrains-mono"
                    data-testid="about-version"
                >
                    {move || version.get()}
                </div>
            </div>

            // ---- Updates --------------------------------------------------
            <div class="py-3.5 border-b border-border space-y-3">
                <div class="flex items-start justify-between gap-5">
                    <div>
                        <div class="text-sm font-medium text-text-primary">
                            {move || t!(i18n, settings.about_updates)}
                        </div>
                        <div class="text-xs text-text-secondary mt-0.5">
                            {move || t!(i18n, settings.about_updates_desc)}
                        </div>
                    </div>
                    <div class="shrink-0 mt-0.5">
                        {move || {
                            let busy = checking.get();
                            view! {
                                <Button
                                    variant=Variant::Secondary
                                    loading=busy
                                    attr:data-testid="about-check-update"
                                    on:click=move |_: web_sys::MouseEvent| check_now()
                                >
                                    {move || t!(i18n, settings.about_check_button)}
                                </Button>
                            }
                        }}
                    </div>
                </div>

                // Inline result — never a modal (④).
                <div data-testid="about-update-result">
                    {move || {
                        match update_status.0.get() {
                            None => ().into_any(),
                            Some(dto) => {
                                match dto.status {
                                    UpdateStatus::UpToDate => {
                                        view! {
                                            <p class="text-xs text-text-secondary">
                                                {move || t!(i18n, settings.about_up_to_date)}
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    UpdateStatus::UpdateAvailable => {
                                        let latest = dto.latest.unwrap_or_default();
                                        let url = dto.release_url.unwrap_or_default();
                                        view! {
                                            <p class="text-xs text-text-primary">
                                                {move || t!(i18n, settings.about_update_available)} " "
                                                <span class="font-jetbrains-mono">{latest}</span> " · "
                                                <a
                                                    class="text-primary hover:underline"
                                                    href=url
                                                    target="_blank"
                                                    rel="noreferrer"
                                                >
                                                    {move || t!(i18n, settings.about_download)}
                                                </a>
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    UpdateStatus::Unknown => {
                                        view! {
                                            <p class="text-xs" style="color:var(--color-danger-text)">
                                                {move || t!(i18n, settings.about_check_failed)}
                                            </p>
                                        }
                                            .into_any()
                                    }
                                }
                            }
                        }
                    }}
                </div>

                // Auto-check toggle.
                <Show when=move || loaded.0.get() fallback=|| ()>
                    <div class="flex items-center justify-between gap-5">
                        <div>
                            <div class="text-sm font-medium text-text-primary">
                                {move || t!(i18n, settings.about_auto_check)}
                            </div>
                            <div class="text-xs text-text-secondary mt-0.5">
                                {move || t!(i18n, settings.about_auto_check_desc)}
                            </div>
                        </div>
                        <div class="shrink-0 mt-0.5">
                            <Toggle
                                checked=Signal::derive(move || sec.0.get().update_check_enabled)
                                on_change=on_auto
                                aria_label=Signal::derive(move || {
                                    t_string!(i18n, settings.about_auto_check).to_owned()
                                })
                            />
                        </div>
                    </div>
                </Show>
            </div>

            // ---- Licence --------------------------------------------------
            <div class="py-3.5 border-b border-border" data-testid="about-licence">
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.about_licence)}
                </div>
                <div class="text-xs text-text-secondary mt-1">
                    {move || t!(i18n, settings.about_licence_body)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.about_copyright)}
                </div>
            </div>

            // ---- Third-party software -------------------------------------
            <div class="py-3.5">
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.about_third_party)}
                </div>
                <div class="text-xs text-text-secondary mt-1 mb-2">
                    {move || t!(i18n, settings.about_third_party_desc)}
                </div>
                <div class="space-y-1" data-testid="about-third-party">
                    {attributions
                        .into_iter()
                        .map(|(id, crates)| {
                            let count = crates.len();
                            view! {
                                <details class="text-xs">
                                    <summary class="cursor-pointer text-text-primary py-1">
                                        {id}" · "{count.to_string()}
                                    </summary>
                                    <ul class="pl-4 py-1 space-y-0.5 text-text-secondary font-jetbrains-mono">
                                        {crates
                                            .into_iter()
                                            .map(|c| view! { <li>{c}</li> })
                                            .collect_view()}
                                    </ul>
                                </details>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        </div>
    }
}

// ---- Attribution parsing -----------------------------------------------------

#[derive(Deserialize)]
struct RawAttribution {
    licenses: Vec<RawGroup>,
}

#[derive(Deserialize)]
struct RawGroup {
    id: String,
    used_by: Vec<RawCrate>,
}

#[derive(Deserialize)]
struct RawCrate {
    name: String,
    version: String,
}

/// Parse the committed cargo-about JSON and merge its per-licence-text groups
/// into one entry per SPDX id, each with a sorted, de-duplicated `name version`
/// crate list. A parse failure degrades to an empty list (the notices still ship
/// in `THIRD-PARTY-NOTICES.txt`).
fn merged_attributions() -> Vec<(String, Vec<String>)> {
    const RAW: &str = include_str!("third-party-licenses.json");
    let Ok(parsed) = serde_json::from_str::<RawAttribution>(RAW) else {
        return Vec::new();
    };
    let mut by_id: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for group in parsed.licenses {
        let set = by_id.entry(group.id).or_default();
        for krate in group.used_by {
            set.insert(format!("{} {}", krate.name, krate.version));
        }
    }
    by_id
        .into_iter()
        .map(|(id, crates)| (id, crates.into_iter().collect()))
        .collect()
}
