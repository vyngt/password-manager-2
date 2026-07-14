//! Backup panel (slice 5.2c; restore removed in 5.2.1) — the Settings → Backup tab.
//!
//! One action: **Back up now** — a live snapshot of the unlocked vault to a `.vbk` archive
//! (save dialog → `backup_vault`), with a readout of the last archive's counts.
//!
//! 🔴 Restore lives on the LAUNCH SCREEN now (slice 5.2.1, Decision ⑦/⑰): a destructive
//! whole-vault operation belongs where the vault is closed, not behind an unlocked Settings
//! tab. The old "lock the vault, restore, then navigate to the launch screen" hack — which
//! was the code telling us the button was in the wrong place — is DELETED. Snapshot revert
//! lives on `/v/snapshots`; disaster restore of a corrupt vault lives on the vault picker.
//!
//! Every `t_string!` is read `untrack`ed (owner-safe from the `spawn_local` futures).

use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::BackupReportDto;
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, SaveDialogOptions};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn BackupPanel() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let busy = RwSignal::new(false);
    let last = RwSignal::new(None::<BackupReportDto>);

    // Owner-safe toast (called from event handlers AND `spawn_local`, so the dismiss label is
    // read via `untrack`, like `vault_launch`'s helpers).
    let show = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, settings.dismiss).to_owned());
        toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
    };

    // ---- Back up now: save dialog → backup_vault ----
    let backup_now = move || {
        if busy.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let dialog_title = untrack(|| t_string!(i18n, settings.backup_now_button).to_owned());
        let saved = untrack(|| t_string!(i18n, settings.backup_saved).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, settings.backup_failed).to_owned());
        busy.set(true);
        spawn_local(async move {
            let opts = SaveDialogOptions {
                title: Some(dialog_title),
                default_path: Some("vedge-backup.vbk".to_owned()),
                filters: vec![DialogFilter {
                    name: "VEdge Backup".to_owned(),
                    extensions: vec!["vbk".to_owned()],
                }],
            };
            match api::dialog::save(&opts).await {
                Ok(Some(dest)) => match api::backup::backup(&path, &dest).await {
                    Ok(report) => {
                        last.set(Some(report));
                        show(saved, ToastVariant::Success);
                    }
                    Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
                },
                // Cancelled — silent.
                Ok(None) => {}
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    view! {
        // ---- Back up now row ----
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.backup_now_title)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.backup_now_desc)}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                {move || {
                    let b = busy.get();
                    view! {
                        <Button
                            variant=Variant::Secondary
                            loading=b
                            attr:data-testid="backup-now"
                            on:click=move |_: web_sys::MouseEvent| backup_now()
                        >
                            {move || t!(i18n, settings.backup_now_button)}
                        </Button>
                    }
                }}
            </div>
        </div>

        // ---- Last-backup report ----
        {move || {
            last.get()
                .map(|r| {
                    view! {
                        <div
                            class="py-3.5 text-xs text-text-secondary space-y-1"
                            data-testid="backup-report"
                        >
                            <div class="font-medium text-text-primary">
                                {move || t!(i18n, settings.backup_report_title)}
                            </div>
                            <ul class="list-disc pl-5 space-y-0.5">
                                <li>
                                    {r.entry_count.to_string()}" "
                                    {move || t!(i18n, settings.backup_report_entries)}
                                </li>
                                <li>
                                    {r.blob_count.to_string()}" "
                                    {move || t!(i18n, settings.backup_report_blobs)}
                                </li>
                                <li>
                                    {r.archive_bytes.to_string()}" "
                                    {move || t!(i18n, settings.backup_report_size)}
                                </li>
                            </ul>
                        </div>
                    }
                })
        }}
    }
}
