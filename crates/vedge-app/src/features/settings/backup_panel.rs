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
use vedge_ipc::{BackupReportDto, BackupStatusDto};
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
    let status = RwSignal::new(None::<BackupStatusDto>);

    // Decision ⑧ — the health readout, and the one place it is easy to get catastrophically
    // wrong. `last_backup_at` is written ONLY by `backup_vault`. It is deliberately NOT derived
    // from `BackupCreated` audit rows, because `create_snapshot` emits those too (5.2.1) — so an
    // audit-derived readout would let a *snapshot* announce "you're backed up".
    //
    // A snapshot lives on the same disk, in the same folder, as the vault it protects. It
    // survives a mistake. It does not survive a dead drive, a stolen laptop, or a ransomware
    // pass. Telling someone they are backed up when they are not is worse than telling them
    // nothing at all — so the two facts are separate columns, with separate writers, and they
    // are shown separately here.
    let load_status = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(s) = api::backup::status(&path).await {
                status.set(Some(s));
            }
        });
    };
    Effect::new(move |_| load_status());

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
                        load_status(); // ⑧ the readout advances only on a REAL backup
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
        // ---- ⑧ Backup health: when were you LAST actually backed up? ----
        <div class="py-3.5 border-b border-border text-xs" data-testid="backup-health">
            <div class="font-medium text-text-primary">
                {move || t!(i18n, settings.backup_health_title)}
            </div>
            <div class="mt-1 space-y-0.5">
                {move || {
                    let s = status.get();
                    let last_backup = s.as_ref().and_then(|s| s.last_backup_at.clone());
                    let last_snapshot = s.and_then(|s| s.last_snapshot_at);
                    let backed_up = last_backup.is_some();
                    let line = last_backup
                        .map_or_else(
                            || t_string!(i18n, settings.backup_health_never).to_owned(),
                            |t| format!("{} {t}", t_string!(i18n, settings.backup_health_last)),
                        );
                    // Not "backed up a while ago" — NEVER backed up. Say it plainly, and in the
                    // warning colour: a snapshot on the same disk is not a backup, and a user who
                    // believes otherwise finds out only when the disk is gone.
                    view! {
                        <div
                            class=if backed_up { "text-text-secondary" } else { "font-medium" }
                            style=if backed_up { "" } else { "color:var(--color-warning-text)" }
                        >
                            {line}
                        </div>
                        // Shown BESIDE the backup line, never instead of it: a snapshot is an
                        // undo, not a backup. It dies with the disk it sits on.
                        {last_snapshot
                            .map(|t| {
                                view! {
                                    <div class="text-text-secondary">
                                        {move || t!(i18n, settings.backup_health_snapshot)}" "{t}
                                    </div>
                                }
                            })}
                    }
                }}
            </div>
        </div>

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
