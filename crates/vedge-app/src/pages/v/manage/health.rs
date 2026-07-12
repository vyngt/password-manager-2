//! Password-health page (`/v/health`) — run the vault-wide secret scan and browse
//! its findings (weak / reused / old). The scan is **on-demand** (a full-vault
//! decrypt is user-initiated) and the report is held in an app-root
//! [`HealthReportCtx`] so it survives navigation and is wiped on lock. Entry names
//! are resolved from the in-memory index with **zero decryption**; the report
//! itself carries only non-invertible derivatives.
//!
//! Data loading follows the house `RwSignal` + `spawn_local` + `Effect` idiom
//! (see `manage/audit.rs`), not `Resource`/`Action`.

use std::collections::{BTreeMap, HashMap, HashSet};

use leptos::either::EitherOf3;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use vedge_ipc::{AuditQueryDto, FindingDto, FindingKindDto, HealthScanInputDto, IndexEntryDto};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::components::foundation::empty_state::EmptyState;
use vedge_ui::components::foundation::spinner::Spinner;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::features::health::context::HealthReportCtx;
use crate::features::health::health_table::HealthTable;
use crate::features::health::summary::HealthSummary;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_view::{clock_time, long_date};
use crate::features::vault::ui_state::VaultUiState;
use crate::i18n::{t_string, use_i18n};

/// Group the Reused findings by their `group` ordinal → the distinct entry names
/// sharing that secret. Sorted by group id. Members are deduped by **`entry_id`**,
/// not display name (two entries may share a name), so the card membership matches
/// the finding row's `count`.
fn reuse_groups(
    findings: &[FindingDto],
    names: &HashMap<String, String>,
) -> Vec<(u32, Vec<String>)> {
    let mut ids: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for f in findings {
        if let FindingKindDto::Reused { group, .. } = f.kind {
            let bucket = ids.entry(group).or_default();
            if !bucket.contains(&f.entry_id) {
                bucket.push(f.entry_id.clone());
            }
        }
    }
    ids.into_iter()
        .map(|(group, entry_ids)| {
            let display = entry_ids
                .iter()
                .map(|id| {
                    names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| id.chars().take(8).collect())
                })
                .collect();
            (group, display)
        })
        .collect()
}

/// Format an RFC-3339 timestamp as `"<long date> <HH:MM:SS>"` (the audit-page
/// convention).
fn fmt_when(ts: &str) -> String {
    format!("{} {}", long_date(ts), clock_time(ts))
}

#[component]
pub fn HealthPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let health = expect_context::<HealthReportCtx>();
    let ui = expect_context::<VaultUiState>();
    let toast = use_toast();

    // Dismiss label read `untrack`ed so this is safe from `spawn_local` futures.
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, health.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let scanning = RwSignal::new(false);
    let index = RwSignal::new(Vec::<IndexEntryDto>::new());
    let last_scanned = RwSignal::new(Option::<String>::None);
    // Generation token: only the newest in-flight scan may apply its result.
    let req_gen = StoredValue::new(0_u32);

    // id -> name, for zero-decrypt entry-name resolution. A `Memo` so the map is
    // rebuilt only when `index` changes.
    let names = Memo::new(move |_| {
        index
            .get()
            .into_iter()
            .map(|e| (e.id, e.name))
            .collect::<HashMap<String, String>>()
    });

    // Load the entry index once per active vault (non-fatal on failure).
    Effect::new(move |_| {
        let path = active.path.get().unwrap_or_default();
        if path.is_empty() {
            index.set(Vec::new());
            return;
        }
        spawn_local(async move {
            if let Ok(list) = api::vault::list_entries(&path).await {
                index.set(list);
            }
        });
    });

    // "Last scanned" comes free from the audit trail (the one HealthScanned row).
    Effect::new(move |_| {
        let path = active.path.get().unwrap_or_default();
        if path.is_empty() {
            last_scanned.set(None);
            return;
        }
        spawn_local(async move {
            let query = AuditQueryDto {
                entry_id: None,
                actions: vec!["HealthScanned".to_owned()],
                since: None,
                until: None,
                limit: 1,
                offset: 0,
            };
            if let Ok(page) = api::audit::list_audit(&path, &query).await {
                last_scanned.set(page.events.first().map(|e| fmt_when(&e.occurred_at)));
            }
        });
    });

    // Run the scan (the slowest operation in the app; explicit button).
    let run_scan = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, health.err_scan).to_owned());
        let token = req_gen.get_value().wrapping_add(1);
        req_gen.set_value(token);
        scanning.set(true);
        spawn_local(async move {
            let result = api::health::scan_health(&path, &HealthScanInputDto::default()).await;
            // A newer scan superseded this one → drop the stale result. `try_get_value`
            // (not `get_value`) because the scan is the app's slowest op: the page may
            // have unmounted mid-flight (sidebar nav / idle auto-lock), disposing this
            // component-scoped token — `get_value` would panic on the disposed value.
            if req_gen.try_get_value() != Some(token) {
                return;
            }
            match result {
                Ok(report) => {
                    last_scanned.set(Some(fmt_when(&report.scanned_at)));
                    health.set(report);
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            scanning.set(false);
        });
    };

    // Click-through: select the entry and jump to the vault view.
    let on_select = Callback::new(move |id: String| {
        ui.selected_id.set(Some(id));
        use_navigate()("/v/vault", Default::default());
    });

    // Derived views over the held report.
    let findings = Signal::derive(move || {
        health
            .report
            .with(|r| r.as_ref().map(|r| r.findings.clone()).unwrap_or_default())
    });
    let summary = Signal::derive(move || {
        health
            .report
            .with(|r| r.as_ref().map(|r| r.summary).unwrap_or_default())
    });
    // Entry-level health gauge inputs: distinct entries with any finding, over
    // entries scanned. Entry-level (not field-level) so the gauge is honest — the
    // per-dimension summary counts double-count a secret hit by several dimensions,
    // and `old` is entry-level anyway.
    let entries_scanned = Signal::derive(move || {
        health
            .report
            .with(|r| r.as_ref().map_or(0, |r| r.entries_scanned))
    });
    let affected_entries = Signal::derive(move || {
        health.report.with(|r| {
            r.as_ref().map_or(0, |r| {
                let distinct: HashSet<&str> =
                    r.findings.iter().map(|f| f.entry_id.as_str()).collect();
                distinct.len() as u32
            })
        })
    });
    let has_report = Signal::derive(move || health.report.with(Option::is_some));
    let has_findings = Signal::derive(move || {
        health
            .report
            .with(|r| r.as_ref().is_some_and(|r| !r.findings.is_empty()))
    });
    let skipped_count = Signal::derive(move || {
        health
            .report
            .with(|r| r.as_ref().map_or(0, |r| r.skipped.len()))
    });
    let reuse = Signal::derive(move || {
        let names = names.get();
        health.report.with(|r| {
            r.as_ref()
                .map_or_else(Vec::new, |r| reuse_groups(&r.findings, &names))
        })
    });

    view! {
        <div class="h-full overflow-y-auto p-6" data-testid="health-page">
            <div class="max-w-5xl mx-auto space-y-4">
                // ---- Header: title + run-scan + last-scanned ----
                <div class="flex flex-wrap items-start justify-between gap-3">
                    <div class="min-w-0">
                        <h1 class="text-xl font-semibold text-text-primary">
                            {move || t_string!(i18n, health.title).to_owned()}
                        </h1>
                        <p class="text-sm text-text-secondary mt-1 max-w-2xl">
                            {move || t_string!(i18n, health.subtitle).to_owned()}
                        </p>
                    </div>
                    <div class="flex flex-col items-end gap-1 shrink-0">
                        {move || {
                            let busy = scanning.get();
                            view! {
                                <Button
                                    variant=Variant::Primary
                                    loading=busy
                                    attr:data-testid="health-run-scan"
                                    on:click=move |_: web_sys::MouseEvent| run_scan()
                                >
                                    {move || t_string!(i18n, health.run_scan).to_owned()}
                                </Button>
                            }
                        }}
                        <span class="text-xs text-text-tertiary" data-testid="health-last-scanned">
                            {move || match last_scanned.get() {
                                Some(when) => {
                                    format!("{} {when}", t_string!(i18n, health.last_scanned))
                                }
                                None => t_string!(i18n, health.never_scanned).to_owned(),
                            }}
                        </span>
                    </div>
                </div>

                // ---- Body ----
                {move || {
                    if scanning.get() {
                        EitherOf3::A(
                            view! {
                                <div class="flex items-center gap-3 text-text-secondary py-10">
                                    <Spinner />
                                    <span>
                                        {move || t_string!(i18n, health.scanning).to_owned()}
                                    </span>
                                </div>
                            },
                        )
                    } else if !has_report.get() {
                        EitherOf3::B(
                            view! {
                                <EmptyState
                                    icon=icondata::FaShieldHalvedSolid
                                    title=Signal::derive(move || {
                                        t_string!(i18n, health.not_scanned_title).to_owned()
                                    })
                                    description=Signal::derive(move || {
                                        t_string!(i18n, health.not_scanned_desc).to_owned()
                                    })
                                />
                            },
                        )
                    } else {
                        EitherOf3::C(
                            view! {
                                <div class="space-y-4">
                                    <HealthSummary
                                        summary=summary
                                        entries_scanned=entries_scanned
                                        affected_entries=affected_entries
                                    />
                                    <Show
                                        when=move || has_findings.get()
                                        fallback=move || {
                                            view! {
                                                <EmptyState
                                                    icon=icondata::FaCircleCheckSolid
                                                    title=Signal::derive(move || {
                                                        t_string!(i18n, health.clean_title).to_owned()
                                                    })
                                                    description=Signal::derive(move || {
                                                        t_string!(i18n, health.clean_desc).to_owned()
                                                    })
                                                />
                                            }
                                        }
                                    >
                                        <HealthTable
                                            findings=findings
                                            names=names
                                            loading=scanning
                                            on_select=on_select
                                        />
                                        <Show when=move || !reuse.with(Vec::is_empty)>
                                            <div class="space-y-2">
                                                <h2 class="text-sm font-semibold text-text-primary">
                                                    {move || {
                                                        t_string!(i18n, health.reuse_heading).to_owned()
                                                    }}
                                                </h2>
                                                <For
                                                    each=move || reuse.get()
                                                    key=|(group, _)| *group
                                                    children=move |(group, entries)| {
                                                        view! {
                                                            <div class="rounded-md border border-secondary/15 p-3">
                                                                <div class="text-xs text-text-secondary mb-1">
                                                                    {format!(
                                                                        "{} #{group}",
                                                                        t_string!(i18n, health.reuse_group),
                                                                    )}
                                                                </div>
                                                                <div class="text-sm text-text-primary">
                                                                    {entries.join(", ")}
                                                                </div>
                                                            </div>
                                                        }
                                                    }
                                                />
                                            </div>
                                        </Show>
                                        <Show when=move || skipped_count.get() != 0>
                                            <p class="text-xs text-text-tertiary">
                                                {move || {
                                                    format!(
                                                        "{} {}",
                                                        skipped_count.get(),
                                                        t_string!(i18n, health.skipped),
                                                    )
                                                }}
                                            </p>
                                        </Show>
                                    </Show>
                                </div>
                            },
                        )
                    }
                }}
            </div>
        </div>
    }
}
