//! Snapshots page (`/v/snapshots`) — the TIME half of Decision ⑦ (slice 5.2.1).
//!
//! Take a local, point-in-time snapshot of this vault, browse the list, delete one, or revert
//! the vault to one. Reverting refuses an *unlocked* target (Windows holds the `.vdb` open), so
//! the page locks the vault first, then reverts, then returns to the launch screen to unlock —
//! and because a revert auto-snapshots first, it is undoable.
//!
//! 🔴 Decision ⑧: a snapshot is NOT a backup. The copy lives next to the vault; the subtitle
//! says so, and the word "backed up" appears nowhere here.
//!
//! Data loading follows the house `RwSignal` + `spawn_local` + `Effect` idiom (see
//! `manage/health.rs`), not `Resource`/`Action`.

use std::collections::HashMap;

use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use vedge_ipc::{ImportActionDto, ImportPreviewRow, SnapshotDto};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::components::foundation::empty_state::EmptyState;
use vedge_ui::primitives::tokens::{ToastVariant, Variant};

use crate::api;
use crate::features::help_popover::HelpPopover;
use crate::features::settings::import_preview_table::ImportPreviewTable;
use crate::features::vault::context::ActiveVault;
use crate::features::vault::entry_view::{clock_time, long_date};
use crate::i18n::{t_string, use_i18n};

fn fmt_when(ts: &str) -> String {
    format!("{} {}", long_date(ts), clock_time(ts))
}

#[component]
pub fn SnapshotsPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();

    let snaps = RwSignal::new(Vec::<SnapshotDto>::new());
    let busy = RwSignal::new(false);
    let reload = RwSignal::new(0_u32);
    // The id of the snapshot whose Revert is awaiting an inline confirm (None = no prompt).
    let pending_revert = RwSignal::new(Option::<String>::None);
    // The tweezers (5.3c): recover individual entries FROM a snapshot into the live vault.
    // One preview at a time; picking + committing reuse the 5.3b import path.
    let preview = RwSignal::new(Vec::<ImportPreviewRow>::new());
    let picked = RwSignal::new(HashMap::<u32, bool>::new());

    // Dismiss label read `untrack`ed so it is safe to use from `spawn_local` futures.
    let notify = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, snapshots.dismiss).to_owned());
        toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
    };

    // Load the snapshot list on mount and after each mutation (bump `reload`).
    Effect::new(move |_| {
        reload.track();
        let path = active.path.get().unwrap_or_default();
        if path.is_empty() {
            snaps.set(Vec::new());
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_list).to_owned());
        spawn_local(async move {
            match api::snapshot::list(&path).await {
                Ok(list) => snaps.set(list),
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    });

    let take = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() || busy.get_untracked() {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_take).to_owned());
        let done = untrack(|| t_string!(i18n, snapshots.taken).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::snapshot::create(&path).await {
                Ok(_) => {
                    notify(done, ToastVariant::Success);
                    reload.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    let delete = move |id: String| {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_delete).to_owned());
        let done = untrack(|| t_string!(i18n, snapshots.delete_done).to_owned());
        spawn_local(async move {
            match api::snapshot::delete(&path, &id).await {
                Ok(()) => {
                    notify(done, ToastVariant::Success);
                    reload.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    };

    // Revert in place: the backend holds the KEK, auto-snapshots first (undoable), swaps, and
    // re-opens. `confirm_rollback = true`: reverting from inside the vault IS the confirmation.
    // 🔴 A revert now LOCKS the vault (`stayed_unlocked = false` on every success) — reverting is
    // a destructive, credentials-affecting operation, so the user re-unlocks against the reverted
    // state instead of silently continuing the pre-revert session. So we navigate to the launch
    // screen. (The `stayed_unlocked` branch is kept defensive in case the backend policy changes.)
    let do_revert = move |id: String| {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_revert).to_owned());
        let done = untrack(|| t_string!(i18n, snapshots.revert_done).to_owned());
        let nav = use_navigate();
        pending_revert.set(None);
        spawn_local(async move {
            match api::snapshot::revert_in_session(&path, &id, true).await {
                Ok(result) => {
                    notify(done, ToastVariant::Warning);
                    if result.stayed_unlocked {
                        // Still unlocked — refresh the list (a pre-restore snapshot was added).
                        reload.update(|n| *n = n.wrapping_add(1));
                    } else {
                        nav("/", Default::default());
                    }
                }
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    };

    // Begin recovering entries from a snapshot: open it with the CURRENT session and load
    // the derivatives-only preview (Decision ⑦). A stale snapshot returns an error whose
    // message is the honest "recover with its original password" copy — surfaced, never a
    // silent no-op. Reuses `begin_snapshot_import` → the shared 5.3b commit path.
    let do_recover = move |id: String| {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() || busy.get_untracked() {
            return;
        }
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_recover).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::snapshot::begin_recover(&path, &id).await {
                Ok(rows) => {
                    // Default: recover everything except rows that cannot be committed.
                    let init: HashMap<u32, bool> = rows
                        .iter()
                        .map(|r| (r.row_id, r.status != "error"))
                        .collect();
                    picked.set(init);
                    preview.set(rows);
                }
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    let recover_commit = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        let rows = preview.get_untracked();
        let pk = picked.get_untracked();
        if rows.is_empty() || path.is_empty() {
            return;
        }
        let actions: Vec<ImportActionDto> = rows
            .iter()
            .map(|r| ImportActionDto {
                row_id: r.row_id,
                import: pk.get(&r.row_id).copied().unwrap_or(false),
            })
            .collect();
        let err_prefix = untrack(|| t_string!(i18n, snapshots.err_recover).to_owned());
        let done = untrack(|| t_string!(i18n, snapshots.recover_done).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::import::commit(&path, &actions).await {
                Ok(rep) => {
                    let variant = if rep.failed.is_empty() {
                        ToastVariant::Success
                    } else {
                        ToastVariant::Warning
                    };
                    notify(format!("{} {done}", rep.imported), variant);
                    preview.set(Vec::new());
                    picked.set(HashMap::new());
                    reload.update(|n| *n = n.wrapping_add(1));
                }
                Err(e) => notify(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    let recover_cancel = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        preview.set(Vec::new());
        picked.set(HashMap::new());
        spawn_local(async move {
            let _ = api::import::cancel(&path).await;
        });
    };

    view! {
        <div class="h-full p-6 flex flex-col" data-testid="snapshots-page">
            <div class="max-w-4xl w-full mx-auto flex flex-col flex-1 min-h-0 gap-4">
                // ---- Header: title + honesty copy + Take snapshot ----
                <div class="flex flex-wrap items-start justify-between gap-3 shrink-0">
                    <div class="min-w-0">
                        <div class="flex items-center gap-1.5">
                            <h1 class="text-xl font-semibold text-text-primary">
                                {move || t_string!(i18n, snapshots.title).to_owned()}
                            </h1>
                            <HelpPopover
                                body=Signal::derive(move || {
                                    t_string!(i18n, snapshots.help_three_way).to_owned()
                                })
                                label=Signal::derive(move || {
                                    t_string!(i18n, settings.help_aria).to_owned()
                                })
                                testid="help-snapshots"
                            />
                        </div>
                        <p class="text-sm text-text-secondary mt-1 max-w-2xl">
                            {move || t_string!(i18n, snapshots.subtitle).to_owned()}
                        </p>
                    </div>
                    {move || {
                        let is_busy = busy.get();
                        view! {
                            <Button
                                variant=Variant::Primary
                                loading=is_busy
                                attr:data-testid="snapshot-take"
                                on:click=move |_: web_sys::MouseEvent| take()
                            >
                                {move || t_string!(i18n, snapshots.take).to_owned()}
                            </Button>
                        }
                    }}
                </div>

                // The scroll band (pattern B): the header above stays fixed; the
                // snapshot list + recover panel own the remaining height and scroll here.
                <div class="flex-1 min-h-0 overflow-y-auto space-y-4">
                    // ---- List, or an empty state ----
                    {move || {
                        let rows = snaps.get();
                        if rows.is_empty() {
                            Either::Left(
                                view! {
                                    <EmptyState
                                        title=Signal::derive(move || {
                                            t_string!(i18n, snapshots.empty_title).to_owned()
                                        })
                                        description=Signal::derive(move || {
                                            t_string!(i18n, snapshots.empty_desc).to_owned()
                                        })
                                    />
                                },
                            )
                        } else {
                            Either::Right(
                                view! {
                                    <ul class="space-y-2" role="list">
                                        <For
                                            each=move || snaps.get()
                                            key=|s| s.id.clone()
                                            children=move |s| {
                                                view! {
                                                    <SnapshotRow
                                                        snap=s
                                                        pending_revert=pending_revert
                                                        on_delete=Callback::new(delete)
                                                        on_revert=Callback::new(do_revert)
                                                        on_recover=Callback::new(do_recover)
                                                    />
                                                }
                                            }
                                        />
                                    </ul>
                                },
                            )
                        }
                    }} // ---- The tweezers: recover picked entries from a snapshot (5.3c) ----
                    <Show when=move || !preview.get().is_empty() fallback=|| ()>
                        <div
                            class="rounded-md border border-border bg-surface-1 p-4 space-y-3"
                            data-testid="recover-panel"
                        >
                            <div class="text-sm font-medium text-text-primary">
                                {move || t_string!(i18n, snapshots.recover_title).to_owned()}
                            </div>
                            <ImportPreviewTable
                                preview=preview
                                picked=picked
                                testid="recover-preview-table"
                            />
                            <div class="flex items-center justify-end gap-2">
                                <Button
                                    variant=Variant::Secondary
                                    on:click=move |_: web_sys::MouseEvent| recover_cancel()
                                >
                                    {move || t_string!(i18n, snapshots.recover_cancel).to_owned()}
                                </Button>
                                {move || {
                                    let b = busy.get();
                                    view! {
                                        <Button
                                            variant=Variant::Primary
                                            loading=b
                                            disabled=b
                                            attr:data-testid="recover-commit"
                                            on:click=move |_: web_sys::MouseEvent| recover_commit()
                                        >
                                            {move || {
                                                t_string!(i18n, snapshots.recover_commit).to_owned()
                                            }}
                                        </Button>
                                    }
                                }}
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
fn SnapshotRow(
    snap: SnapshotDto,
    pending_revert: RwSignal<Option<String>>,
    on_delete: Callback<String>,
    on_revert: Callback<String>,
    on_recover: Callback<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let id = snap.id.clone();
    let is_pre_restore = snap.reason == "pre-restore";
    let stale = snap.stale_credential;
    let when = fmt_when(&snap.created_at);
    let entry_count = snap.entry_count;
    let blob_count = snap.blob_count;

    let id_revert = id.clone();
    let id_confirm = id.clone();
    let id_delete = id.clone();
    let id_recover = id.clone();
    let id_pending = id;
    let awaiting = move || pending_revert.get().as_deref() == Some(id_pending.as_str());

    view! {
        <li
            class="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border-subtle bg-surface-1 px-4 py-3"
            data-testid="snapshot-row"
        >
            <div class="min-w-0">
                <div class="flex items-center gap-2 flex-wrap">
                    <span class="text-sm font-medium text-text-primary">{when}</span>
                    <span class="text-xs text-text-tertiary">
                        {move || {
                            if is_pre_restore {
                                t_string!(i18n, snapshots.reason_pre_restore).to_owned()
                            } else {
                                t_string!(i18n, snapshots.reason_manual).to_owned()
                            }
                        }}
                    </span>
                    {stale
                        .then(|| {
                            view! {
                                <span
                                    class="text-xs text-warning-text"
                                    title=move || t_string!(i18n, snapshots.stale_hint).to_owned()
                                >
                                    {move || {
                                        format!("⚠ {}", t_string!(i18n, snapshots.stale_badge))
                                    }}
                                </span>
                            }
                        })}
                </div>
                <div class="text-xs text-text-secondary mt-0.5">
                    {move || {
                        format!(
                            "{entry_count} {} · {blob_count} {}",
                            t_string!(i18n, snapshots.entries),
                            t_string!(i18n, snapshots.blobs),
                        )
                    }}
                </div>
            </div>

            <div class="flex items-center gap-2 shrink-0">
                {move || {
                    if awaiting() {
                        let id_c = id_confirm.clone();
                        Either::Left(
                            view! {
                                <span class="text-xs text-text-secondary">
                                    {move || {
                                        t_string!(i18n, snapshots.revert_confirm_title).to_owned()
                                    }}
                                </span>
                                <Button
                                    variant=Variant::Ghost
                                    on:click=move |_: web_sys::MouseEvent| pending_revert.set(None)
                                >
                                    {move || t_string!(i18n, snapshots.revert_cancel).to_owned()}
                                </Button>
                                <Button
                                    variant=Variant::Warning
                                    attr:data-testid="snapshot-revert-confirm"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        on_revert.run(id_c.clone());
                                    }
                                >
                                    {move || t_string!(i18n, snapshots.revert).to_owned()}
                                </Button>
                            },
                        )
                    } else {
                        let id_r = id_revert.clone();
                        let id_d = id_delete.clone();
                        let id_rec = id_recover.clone();
                        Either::Right(
                            view! {
                                <Button
                                    variant=Variant::Secondary
                                    attr:data-testid="snapshot-recover"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        on_recover.run(id_rec.clone());
                                    }
                                >
                                    {move || t_string!(i18n, snapshots.recover).to_owned()}
                                </Button>
                                <Button
                                    variant=Variant::Secondary
                                    attr:data-testid="snapshot-revert"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        pending_revert.set(Some(id_r.clone()));
                                    }
                                >
                                    {move || t_string!(i18n, snapshots.revert).to_owned()}
                                </Button>
                                <Button
                                    variant=Variant::Danger
                                    attr:data-testid="snapshot-delete"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        on_delete.run(id_d.clone());
                                    }
                                >
                                    {move || t_string!(i18n, snapshots.delete).to_owned()}
                                </Button>
                            },
                        )
                    }
                }}
            </div>
        </li>
    }
}
