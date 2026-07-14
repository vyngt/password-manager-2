//! Backup panel (slice 5.2c) — the Settings → Backup tab.
//!
//! Two actions over the already-registered backup/restore commands:
//! - **Back up now** — a live snapshot of the unlocked vault to a `.vbk` archive
//!   (save dialog → `backup_vault`), with a readout of the last archive's counts.
//! - **Restore from backup** — pick a `.vbk` → `inspect_backup` → a confirmation
//!   `Dialog` showing the diff + rollback framing (or a *blocked* state for a
//!   uuid/format/schema mismatch). Restore refuses an *unlocked* target
//!   (Decision ⑦), so the confirm handler **locks the vault, restores, then
//!   navigates to the launch screen** — the vault is being replaced anyway.
//!
//! Every `t_string!` is read `untrack`ed (owner-safe from the `spawn_local`
//! futures); the dialog's interpolated lines use view-context `t!`.

use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use vedge_ipc::{BackupPreviewDto, BackupReportDto};
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

use crate::api;
use crate::api::dialog::{DialogFilter, OpenDialogOptions, SaveDialogOptions};
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_view::long_date;
use crate::i18n::{t, t_string, use_i18n};

#[component]
pub fn BackupPanel() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let busy = RwSignal::new(false);
    let last = RwSignal::new(None::<BackupReportDto>);
    // (archive_path, preview) — `Some` drives the restore confirmation dialog.
    let preview = RwSignal::new(None::<(String, BackupPreviewDto)>);
    let restoring = RwSignal::new(false);

    // Owner-safe toast (called from event handlers AND `spawn_local`, so the dismiss
    // label is read via `untrack`, like `vault_launch`'s helpers).
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

    // ---- Open restore picker → inspect → preview dialog ----
    let open_restore = move || {
        if restoring.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let dialog_title = untrack(|| t_string!(i18n, settings.restore_button).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, settings.restore_failed).to_owned());
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(dialog_title),
                filters: vec![DialogFilter {
                    name: "VEdge Backup".to_owned(),
                    extensions: vec!["vbk".to_owned()],
                }],
                directory: false,
            };
            match api::dialog::open(&opts).await {
                Ok(Some(archive)) => match api::backup::inspect(&path, &archive).await {
                    Ok(p) => preview.set(Some((archive, p))),
                    Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
                },
                Ok(None) => {}
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    };

    // ---- Confirm restore: lock the vault → restore → go to the launch screen ----
    let do_restore = move || {
        let Some((archive, _)) = preview.get_untracked() else {
            return;
        };
        if restoring.get_untracked() {
            return;
        }
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let done = untrack(|| t_string!(i18n, settings.restore_done).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, settings.restore_failed).to_owned());
        let nav = use_navigate();
        restoring.set(true);
        spawn_local(async move {
            // Restore refuses an unlocked target (Decision ⑦) — lock this vault first;
            // it is being replaced anyway. Best-effort (already-locked is fine).
            api::vault::lock(&path).await.ok();
            match api::backup::restore(&path, &archive, true).await {
                Ok(_) => {
                    preview.set(None);
                    show(done, ToastVariant::Warning);
                    nav("/", Default::default());
                }
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            restoring.set(false);
        });
    };

    let close_preview = Callback::new(move |()| preview.set(None));

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

        // ---- Restore row ----
        <div class="flex items-start justify-between gap-5 py-3.5 border-b border-border">
            <div>
                <div class="text-sm font-medium text-text-primary">
                    {move || t!(i18n, settings.restore_title)}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || t!(i18n, settings.restore_desc)}
                </div>
            </div>
            <div class="shrink-0 mt-0.5">
                <Button
                    variant=Variant::Secondary
                    attr:data-testid="restore-open"
                    on:click=move |_: web_sys::MouseEvent| open_restore()
                >
                    {move || t!(i18n, settings.restore_button)}
                </Button>
            </div>
        </div>

        // ---- Restore preview dialog (always mounted; open iff a preview is set) ----
        <Dialog
            open=Signal::derive(move || preview.get().is_some())
            on_close=close_preview
            size=DialogSize::Sm
            close_label=Signal::derive(move || t_string!(i18n, settings.restore_cancel).to_owned())
        >
            <DialogHeader>
                <DialogTitle>{move || t!(i18n, settings.restore_dialog_title)}</DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-3 pb-6 min-w-[22rem]">
                    {move || {
                        preview
                            .get()
                            .map(|(_, p)| {
                                let blocked = p.uuid_mismatch || p.unknown_format
                                    || p.unknown_schema;
                                let backup_entries = p.backup_entry_count;
                                let created = long_date(&p.created_at);
                                let target_entries = p.target_entry_count;
                                let rollback = p.rollback_delta;
                                let uuid_mismatch = p.uuid_mismatch;
                                let unknown_format = p.unknown_format;
                                view! {
                                    <p class="text-sm text-text-primary">
                                        {t!(
                                            i18n, settings.restore_preview_backup, entries =
                                            backup_entries, date = created
                                        )}
                                    </p>
                                    {target_entries
                                        .map(|te| {
                                            view! {
                                                <p class="text-sm text-text-secondary">
                                                    {t!(i18n, settings.restore_preview_target, entries = te)}
                                                </p>
                                            }
                                        })}
                                    {rollback
                                        .map(|d| {
                                            view! {
                                                <p class="text-sm text-warning-text">
                                                    {t!(i18n, settings.restore_rollback_warning, delta = d)}
                                                </p>
                                            }
                                        })}
                                    {if blocked {
                                        Either::Left(
                                            view! {
                                                <p class="text-sm text-danger-text">
                                                    {if uuid_mismatch {
                                                        EitherOf3::A(t!(i18n, settings.restore_blocked_uuid))
                                                    } else if unknown_format {
                                                        EitherOf3::B(t!(i18n, settings.restore_blocked_format))
                                                    } else {
                                                        EitherOf3::C(t!(i18n, settings.restore_blocked_schema))
                                                    }}
                                                </p>
                                                <div class="flex justify-end">
                                                    <Button
                                                        variant=Variant::Ghost
                                                        size=Size::Sm
                                                        on:click=move |_: web_sys::MouseEvent| close_preview.run(())
                                                    >
                                                        {move || t!(i18n, settings.restore_cancel)}
                                                    </Button>
                                                </div>
                                            },
                                        )
                                    } else {
                                        Either::Right(
                                            view! {
                                                <div class="flex justify-end gap-2">
                                                    <Button
                                                        variant=Variant::Ghost
                                                        size=Size::Sm
                                                        on:click=move |_: web_sys::MouseEvent| close_preview.run(())
                                                    >
                                                        {move || t!(i18n, settings.restore_cancel)}
                                                    </Button>
                                                    <Button
                                                        variant=Variant::Danger
                                                        size=Size::Sm
                                                        attr:data-testid="restore-confirm"
                                                        on:click=move |_: web_sys::MouseEvent| do_restore()
                                                    >
                                                        {move || t!(i18n, settings.restore_confirm)}
                                                    </Button>
                                                </div>
                                            },
                                        )
                                    }}
                                }
                            })
                    }}
                </div>
            </DialogBody>
        </Dialog>
    }
}
