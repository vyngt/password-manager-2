//! Maintenance panel (slice 4.6a) — the Settings → Maintenance tab.
//!
//! A user-initiated "Run now" for the cleanup pass that otherwise runs on the
//! shell scheduler's 6 h job (purge trashed entries past retention, trim the audit
//! log, reap orphaned document blobs), plus a readout of the last run's counts.
//!
//! The "last report" shown is the last **manual** run in this session (the
//! background 6 h runs only log); no shared last-report state is threaded through
//! the backend — DRIFT from the spec's "showing the last report", noted.

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::MaintenanceReportDto;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn MaintenancePanel() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let running = RwSignal::new(false);
    let last = RwSignal::new(None::<MaintenanceReportDto>);

    let run_now = move || {
        if running.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        // Hoist the localized strings out of the async block (owner-safe).
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        let ok_msg = untrack(|| t_string!(i18n, settings.maintenance_ran).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, settings.maintenance_failed).to_owned());
        running.set(true);
        spawn_local(async move {
            let result = api::maintenance::run_maintenance(&path).await;
            running.set(false);
            match result {
                Ok(report) => {
                    last.set(Some(report));
                    toast.show(
                        ToastInput::new(ok_msg)
                            .variant(ToastVariant::Success)
                            .dismiss_label(dismiss),
                    );
                }
                Err(e) => {
                    toast.show(
                        ToastInput::new(format!("{err_prefix}{e}"))
                            .variant(ToastVariant::Danger)
                            .dismiss_label(dismiss),
                    );
                }
            }
        });
    };

    view! {
        // ---- Run now row ----------------------------------------------
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.maintenance_title)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.maintenance_desc)}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                {move || {
                    let busy = running.get();
                    view! {
                        <Button
                            variant=Variant::Secondary
                            loading=busy
                            attr:data-testid="maintenance-run-now"
                            on:click=move |_: web_sys::MouseEvent| run_now()
                        >
                            {move || t!(i18n, settings.maintenance_run_now)}
                        </Button>
                    }
                }}
            </div>
        </div>

        // ---- Last-run report ------------------------------------------
        {move || {
            last.get()
                .map(|r| {
                    view! {
                        <div class="py-3.5 text-xs text-text-secondary space-y-1">
                            <div class="font-medium text-text-primary">
                                {move || t!(i18n, settings.maintenance_last_run)}
                            </div>
                            <ul class="list-disc pl-5 space-y-0.5" data-testid="maintenance-report">
                                <li>
                                    {r.trashed_entries_deleted.to_string()}" "
                                    {move || t!(i18n, settings.maintenance_trashed)}
                                </li>
                                <li>
                                    {r.audit_events_deleted.to_string()}" "
                                    {move || t!(i18n, settings.maintenance_audit)}
                                </li>
                                <li>
                                    {r.orphaned_blobs_deleted.to_string()}" "
                                    {move || t!(i18n, settings.maintenance_blobs)}
                                </li>
                            </ul>
                        </div>
                    }
                })
        }}
    }
}
