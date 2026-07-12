//! The scan summary header: per-kind count badges + an overall-health bar.
//! Composed from `Badge` + `ProgressBar` — no new `vedge-ui` component (the
//! exemptions are shown as a "Not scored" badge so they are visible, not hidden).

use leptos::prelude::*;
use vedge_ipc::HealthSummaryDto;
use vedge_ui::components::foundation::badge::Badge;
use vedge_ui::components::foundation::progress_bar::ProgressBar;
use vedge_ui::primitives::tokens::{BadgeVariant, ProgressVariant};

use crate::i18n::{t_string, use_i18n};

/// Overall-health percentage (0–100): the share of scanned **entries** with no
/// finding. Entry-level (not a sum of per-dimension counts): the summary's
/// `weak`/`reused`/`old` totals double-count an entry hit by several dimensions,
/// so summing them would understate health. `entries_scanned == 0` (empty vault)
/// reads as fully healthy.
fn health_pct(entries_scanned: u32, affected_entries: u32) -> f64 {
    if entries_scanned == 0 {
        return 100.0;
    }
    let affected = affected_entries.min(entries_scanned);
    let healthy = entries_scanned - affected;
    f64::from(healthy) * 100.0 / f64::from(entries_scanned)
}

#[component]
pub fn HealthSummary(
    #[prop(into)] summary: Signal<HealthSummaryDto>,
    /// Total entries the scan processed (the health-gauge denominator).
    #[prop(into)]
    entries_scanned: Signal<u32>,
    /// Distinct entries with at least one finding (the health-gauge numerator).
    #[prop(into)]
    affected_entries: Signal<u32>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pct = Signal::derive(move || health_pct(entries_scanned.get(), affected_entries.get()));

    view! {
        <div class="space-y-3" data-testid="health-summary">
            <div class="flex flex-wrap items-center gap-2">
                {move || {
                    let s = summary.get();
                    view! {
                        <Badge variant=BadgeVariant::Danger>
                            {format!("{}: {}", t_string!(i18n, health.summary_weak), s.weak)}
                        </Badge>
                        <Badge variant=BadgeVariant::Warning>
                            {format!("{}: {}", t_string!(i18n, health.summary_reused), s.reused)}
                        </Badge>
                        <Badge variant=BadgeVariant::Info>
                            {format!("{}: {}", t_string!(i18n, health.summary_old), s.old)}
                        </Badge>
                        <Badge variant=BadgeVariant::Default>
                            {format!(
                                "{}: {}",
                                t_string!(i18n, health.summary_exempt),
                                s.exempt_not_scored,
                            )}
                        </Badge>
                    }
                }}
            </div>
            {move || {
                let v = pct.get();
                let variant = if v >= 80.0 {
                    ProgressVariant::Success
                } else if v >= 40.0 {
                    ProgressVariant::Default
                } else {
                    ProgressVariant::Danger
                };
                view! {
                    <ProgressBar
                        value=pct
                        variant=variant
                        show_value=true
                        label=Signal::derive(move || {
                            t_string!(i18n, health.health_label).to_owned()
                        })
                    />
                }
            }}
        </div>
    }
}
