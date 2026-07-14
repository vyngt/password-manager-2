//! **Open a backup** — and, behind an Advanced disclosure, **Replace this vault** (slice 5.2.2).
//!
//! Lives on the **launch screen**, because both verbs act on a vault that is *closed*. (5.2's
//! restore lived in Settings, which needs an unlocked vault — so the one vault you most needed
//! to restore, a corrupt one, was the one you could not reach. That was finding H0.)
//!
//! # Two verbs, one dialog, and the difference is the whole point
//!
//! | | **Open backup** (default) | **Advanced ▸ Replace** |
//! |---|---|---|
//! | destination | a path with **nothing there** | a **live vault** you already have |
//! | destructive | 🟢 never — an occupied path is refused | 🔴 yes |
//! | confirmations | none needed | up to **three**, each its own |
//!
//! Open is the default and the safe one. Replace is collapsed, red, and never the first thing a
//! frightened user sees — it exists only because a very large vault cannot afford the transient
//! second copy that Open needs.
//!
//! # Two steps, so the preview always precedes the act
//!
//! Choose → **Preview** → act. The preview is fetched once, deliberately, rather than on every
//! keystroke: the user must see the counts, the date, and the credential warning *before* the
//! button that does something appears. On the Replace path the preview is also where the three
//! risks are disclosed, each with its own checkbox — because a user who has understood that this
//! rolls their vault back has not thereby understood that it needs a password they may no longer
//! have.
//!
//! The archive path is a typeable `Input` **with** a Browse button, exactly as the create
//! wizard's Location field is (slice 5.2.0). Paths are pasteable, which is a real affordance —
//! and it is also the only seam a `WebDriver` can drive, since a native file dialog cannot be
//! automated.
//!
//! Reactivity: every `t_string!` used inside a `spawn_local` is read **before** the async block
//! (an owner-less future would panic or freeze at first render — see CLAUDE.md).

use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uuid::Uuid;

use vedge_ipc::{BackupPreviewDto, RecentVaultDto, TargetStateDto};

use crate::api;
use crate::api::dialog::{DialogFilter, OpenDialogOptions};
use crate::features::vault::home_path::compose_home;
use crate::features::vault::vault_launch::Selected;
use crate::i18n::{t, t_string, use_i18n};

use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::{Button, Checkbox, Dialog, Input};
use vedge_ui::primitives::tokens::{DialogSize, Size, ToastVariant, Variant};

/// Which verb the dialog is currently offering.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Materialise the backup at a fresh path. Cannot destroy anything.
    Open,
    /// 🔴 Overwrite a live vault. Every guard, every confirmation.
    Replace,
}

/// A refusal the backend WILL make. The preview surfaces it so the user is not told "no" only
/// after committing to the action — but the backend re-checks every one of these itself.
struct HardStops {
    blocked: bool,
    reason: Option<&'static str>,
}

#[component]
pub fn BackupOpenDialog(
    open: RwSignal<bool>,
    /// The recents row currently highlighted — the Replace target. Replace is unavailable
    /// without one; there is nothing to replace.
    selected: RwSignal<Option<Selected>>,
    /// Called after a successful open/replace so the picker reloads.
    on_done: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let toast = use_toast();

    let mode = RwSignal::new(Mode::Open);
    let archive = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let location = RwSignal::new(String::new());
    let preview = RwSignal::new(None::<BackupPreviewDto>);
    let busy = RwSignal::new(false);

    // ③ Three independent risks ⇒ three independent acknowledgements. Never one checkbox for
    // two of them.
    let confirm_rollback = RwSignal::new(false);
    let confirm_credentials = RwSignal::new(false);
    let confirm_unverified = RwSignal::new(false);

    // Owner-safe toast: called from event handlers AND `spawn_local` futures.
    let show = move |msg: String, variant: ToastVariant| {
        let dismiss = untrack(|| t_string!(i18n, unlock.dismiss).to_owned());
        toast.show(ToastInput::new(msg).variant(variant).dismiss_label(dismiss));
    };

    // The copy for a hard stop. A closure (not a free fn) so it captures `i18n` without needing
    // to name its generated type, and so the `t_string!` read stays inside a reactive owner.
    let blocked_message = move |reason: Option<&'static str>| -> String {
        match reason {
            Some("format") => t_string!(i18n, unlock.ob_blocked_format).to_owned(),
            Some("schema") => t_string!(i18n, unlock.ob_blocked_schema).to_owned(),
            Some("occupied") => t_string!(i18n, unlock.ob_blocked_occupied).to_owned(),
            Some("wrong_vault") => t_string!(i18n, unlock.ob_blocked_wrong_vault).to_owned(),
            Some("no_target") => t_string!(i18n, unlock.ob_blocked_no_target).to_owned(),
            _ => String::new(),
        }
    };

    // Everything back to its initial state. 🔴 A reopened dialog must never carry a stale
    // preview — and above all never a stale *acknowledgement*. A checkbox the user ticked for
    // one archive must not silently still be ticked for the next one.
    let reset = move || {
        mode.set(Mode::Open);
        archive.set(String::new());
        name.set(String::new());
        location.set(String::new());
        preview.set(None);
        confirm_rollback.set(false);
        confirm_credentials.set(false);
        confirm_unverified.set(false);
        busy.set(false);
    };

    let close = move || {
        reset();
        open.set(false);
    };

    // The composed destination home, live: `<location>/<name>.vedge`.
    let dest_home = Signal::derive(move || compose_home(&location.get(), &name.get()));

    // The Replace target = the highlighted recents row.
    let target_path = Signal::derive(move || selected.get().map(|s| s.path));

    // Ready to ask the backend for a preview?
    let can_preview = Signal::derive(move || {
        !archive.get().trim().is_empty()
            && match mode.get() {
                Mode::Open => !dest_home.get().is_empty(),
                Mode::Replace => target_path.get().is_some(),
            }
    });

    // ---- Browse for the `.vbk` (the first file-with-filter picker in the app) ----
    let browse_archive = move || {
        let title = untrack(|| t_string!(i18n, unlock.ob_archive_dialog).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_open).to_owned());
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(title),
                filters: vec![DialogFilter {
                    name: "VEdge Backup".to_owned(),
                    extensions: vec!["vbk".to_owned()],
                }],
                directory: false,
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    archive.set(path);
                    preview.set(None); // a new archive invalidates any preview
                }
                Ok(None) => {}
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    };

    // ---- Browse for the destination's parent folder ----
    let browse_location = move || {
        let title = untrack(|| t_string!(i18n, onboarding.choose_dialog_title).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, unlock.err_open).to_owned());
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some(title),
                filters: vec![],
                directory: true,
            };
            match api::dialog::open(&opts).await {
                Ok(Some(path)) => {
                    location.set(path);
                    preview.set(None);
                }
                Ok(None) => {}
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
        });
    };

    // ---- Step 1 → 2: fetch the preview ----
    let load_preview = move || {
        if busy.get_untracked() || !can_preview.get_untracked() {
            return;
        }
        let archive_path = archive.get_untracked().trim().to_owned();
        let is_open = mode.get_untracked() == Mode::Open;
        let dest = dest_home.get_untracked();
        let target = target_path.get_untracked();
        let err_prefix = untrack(|| t_string!(i18n, unlock.ob_err_preview).to_owned());
        busy.set(true);
        spawn_local(async move {
            let (t, d) = if is_open {
                (None, Some(dest.as_str()))
            } else {
                (target.as_deref(), None)
            };
            match api::backup::inspect(&archive_path, t, d).await {
                Ok(p) => preview.set(Some(p)),
                Err(e) => show(format!("{err_prefix}{e}"), ToastVariant::Danger),
            }
            busy.set(false);
        });
    };

    // ---- Commit: Open ----
    let do_open = move || {
        if busy.get_untracked() {
            return;
        }
        let archive_path = archive.get_untracked().trim().to_owned();
        let dest = dest_home.get_untracked();
        let label = name.get_untracked().trim().to_owned();
        if archive_path.is_empty() || dest.is_empty() {
            return;
        }
        let done = untrack(|| t_string!(i18n, unlock.ob_opened).to_owned());
        let copied = untrack(|| t_string!(i18n, unlock.ob_opened_copy).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, unlock.ob_err_open).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::backup::open(&archive_path, &dest).await {
                Ok(report) => {
                    // Register it so it shows up in the picker. A duplicate got a fresh identity,
                    // so say so — the user made a copy, and it is now its own vault.
                    let dto = RecentVaultDto {
                        id: Uuid::new_v4().to_string(),
                        path: report.home.clone(),
                        display_name: label,
                        last_opened: None,
                        sort_order: 0,
                    };
                    if let Err(e) = api::recent::add_recent_vault(&dto).await {
                        show(format!("{err_prefix}{e}"), ToastVariant::Danger);
                    } else if report.fresh_uuid {
                        show(copied, ToastVariant::Warning);
                    } else {
                        show(done, ToastVariant::Success);
                    }
                    on_done.run(());
                    close();
                }
                Err(e) => {
                    show(format!("{err_prefix}{e}"), ToastVariant::Danger);
                    busy.set(false);
                }
            }
        });
    };

    // ---- Commit: Replace (destructive) ----
    let do_replace = move || {
        if busy.get_untracked() {
            return;
        }
        let archive_path = archive.get_untracked().trim().to_owned();
        let Some(target) = target_path.get_untracked() else {
            return;
        };
        let (rb, cred, unver) = (
            confirm_rollback.get_untracked(),
            confirm_credentials.get_untracked(),
            confirm_unverified.get_untracked(),
        );
        let done = untrack(|| t_string!(i18n, unlock.ob_replaced).to_owned());
        let undoable = untrack(|| t_string!(i18n, unlock.ob_replaced_undo).to_owned());
        let err_prefix = untrack(|| t_string!(i18n, unlock.ob_err_replace).to_owned());
        busy.set(true);
        spawn_local(async move {
            match api::backup::replace(&target, &archive_path, rb, cred, unver).await {
                Ok(report) => {
                    // ⑭ tell them it is undoable, and where the undo lives. A destructive action
                    // the user knows they can walk back is a different experience entirely.
                    let msg = if report.undo_snapshot_id.is_some() {
                        format!("{done} {undoable}")
                    } else {
                        done
                    };
                    show(msg, ToastVariant::Success);
                    on_done.run(());
                    close();
                }
                Err(e) => {
                    show(format!("{err_prefix}{e}"), ToastVariant::Danger);
                    busy.set(false);
                }
            }
        });
    };

    view! {
        <Dialog
            open=open
            size=DialogSize::Md
            on_close=Callback::new(move |()| close())
            close_label=Signal::derive(move || t_string!(i18n, unlock.ob_cancel).to_owned())
        >
            <div class="space-y-4" data-testid="backup-open-dialog">
                <div>
                    <h2 class="text-base font-semibold text-text-primary">
                        {move || t!(i18n, unlock.ob_title)}
                    </h2>
                    <p class="text-xs text-text-secondary mt-0.5">
                        {move || t!(i18n, unlock.ob_subtitle)}
                    </p>
                </div>

                // ---- The archive: typed, pasteable, with a Browse button ----
                <div class="space-y-1">
                    <label class="text-xs font-medium text-text-primary" for="backup-archive">
                        {move || t!(i18n, unlock.ob_archive_label)}
                    </label>
                    <div class="flex items-end gap-2">
                        <div class="flex-1">
                            <Input
                                id="backup-archive"
                                value=Signal::derive(move || archive.get())
                                placeholder=Signal::derive(move || {
                                    t_string!(i18n, unlock.ob_archive_placeholder).to_owned()
                                })
                                on_input=Callback::new(move |v: String| {
                                    archive.set(v);
                                    preview.set(None);
                                })
                            />
                        </div>
                        <Button
                            variant=Variant::Secondary
                            size=Size::Md
                            attr:data-testid="backup-archive-browse"
                            on:click=move |_: web_sys::MouseEvent| browse_archive()
                        >
                            {move || t!(i18n, onboarding.choose)}
                        </Button>
                    </div>
                </div>

                // ---- Where it goes: a fresh home (Open) or the selected vault (Replace) ----
                {move || {
                    if mode.get() == Mode::Open {
                        Either::Left(
                            view! {
                                <div class="space-y-3">
                                    <div class="space-y-1">
                                        <label
                                            class="text-xs font-medium text-text-primary"
                                            for="backup-name"
                                        >
                                            {move || t!(i18n, unlock.ob_name_label)}
                                        </label>
                                        <Input
                                            id="backup-name"
                                            value=Signal::derive(move || name.get())
                                            placeholder=Signal::derive(move || {
                                                t_string!(i18n, unlock.ob_name_placeholder).to_owned()
                                            })
                                            on_input=Callback::new(move |v: String| {
                                                name.set(v);
                                                preview.set(None);
                                            })
                                        />
                                    </div>
                                    <div class="space-y-1">
                                        <label
                                            class="text-xs font-medium text-text-primary"
                                            for="backup-location"
                                        >
                                            {move || t!(i18n, unlock.ob_location_label)}
                                        </label>
                                        <div class="flex items-end gap-2">
                                            <div class="flex-1">
                                                <Input
                                                    id="backup-location"
                                                    value=Signal::derive(move || location.get())
                                                    placeholder=Signal::derive(move || {
                                                        t_string!(i18n, onboarding.location_placeholder).to_owned()
                                                    })
                                                    on_input=Callback::new(move |v: String| {
                                                        location.set(v);
                                                        preview.set(None);
                                                    })
                                                />
                                            </div>
                                            <Button
                                                variant=Variant::Secondary
                                                size=Size::Md
                                                attr:data-testid="backup-location-browse"
                                                on:click=move |_: web_sys::MouseEvent| { browse_location() }
                                            >
                                                {move || t!(i18n, onboarding.choose)}
                                            </Button>
                                        </div>
                                    </div>
                                    // The live preview of the home that will be created.
                                    <div
                                        class="rounded border border-border bg-surface-muted px-2.5 py-2 text-xs"
                                        data-testid="backup-dest-preview"
                                    >
                                        <span class="text-text-secondary">
                                            {move || t!(i18n, onboarding.preview_label)}
                                        </span>
                                        <span class="ml-1 font-mono text-text-primary">
                                            {move || {
                                                let h = dest_home.get();
                                                if h.is_empty() {
                                                    untrack(|| {
                                                        t_string!(i18n, onboarding.preview_placeholder).to_owned()
                                                    })
                                                } else {
                                                    h
                                                }
                                            }}
                                        </span>
                                    </div>
                                </div>
                            },
                        )
                    } else {
                        Either::Right(
                            view! {
                                <div
                                    class="rounded border px-2.5 py-2 text-xs"
                                    style="border-color:var(--color-danger);background:var(--color-danger-muted)"
                                    data-testid="replace-target"
                                >
                                    <div class="font-medium" style="color:var(--color-danger-text)">
                                        {move || t!(i18n, unlock.ob_replace_target)}
                                    </div>
                                    <div class="mt-0.5 font-mono text-text-primary">
                                        {move || {
                                            target_path
                                                .get()
                                                .unwrap_or_else(|| {
                                                    untrack(|| {
                                                        t_string!(i18n, unlock.ob_replace_no_target).to_owned()
                                                    })
                                                })
                                        }}
                                    </div>
                                </div>
                            },
                        )
                    }
                }}

                // ---- The preview, and the acknowledgements ----
                {move || {
                    preview
                        .get()
                        .map(|p| {
                            let stops = hard_stops(&p, mode.get());
                            view! {
                                <div
                                    class="rounded border border-border px-2.5 py-2 text-xs space-y-1.5"
                                    data-testid="backup-preview"
                                >
                                    <div class="text-text-primary">
                                        {p.backup_entry_count.to_string()}" "
                                        {move || t!(i18n, unlock.ob_entries)}" · "
                                        {p.created_at.clone()}
                                    </div>

                                    // 🔴 A hard stop: no checkbox will get past this.
                                    {stops
                                        .reason
                                        .map(|_| {
                                            view! {
                                                <div
                                                    class="font-medium"
                                                    style="color:var(--color-danger-text)"
                                                    data-testid="backup-blocked"
                                                >
                                                    {move || blocked_message(stops.reason)}
                                                </div>
                                            }
                                        })}

                                    // ② This is a copy — and it will get its own identity.
                                    {p
                                        .duplicate_of
                                        .clone()
                                        .map(|_| {
                                            view! {
                                                <div
                                                    class="text-text-secondary"
                                                    data-testid="backup-duplicate"
                                                >
                                                    {move || t!(i18n, unlock.ob_duplicate)}
                                                </div>
                                            }
                                        })}

                                    // M3: the credential story — and NEVER a false "same".
                                    // 🔴 `Some(false)` renders NOTHING. Silence here means
                                    // "checked, and they match" — and that is the only case in
                                    // which we are allowed to be silent. `None` is *unknown* and
                                    // must SAY it is unknown; a pre-5.2.2 archive carries no
                                    // credential prefix, and quietly treating unknown as "fine"
                                    // is how a user shreds the only kit that still opens it.
                                    {(p.credentials_differ != Some(false))
                                        .then(|| {
                                            let differs = p.credentials_differ == Some(true);
                                            let cls = if differs {
                                                "font-medium"
                                            } else {
                                                "text-text-secondary"
                                            };
                                            let style = if differs {
                                                "color:var(--color-warning-text)"
                                            } else {
                                                ""
                                            };
                                            view! {
                                                <div class=cls style=style data-testid="backup-credentials">
                                                    {move || {
                                                        if differs {
                                                            t_string!(i18n, unlock.ob_creds_differ).to_owned()
                                                        } else {
                                                            t_string!(i18n, unlock.ob_creds_unknown).to_owned()
                                                        }
                                                    }}
                                                </div>
                                            }
                                        })}

                                    // ③ Replace only: three risks, three checkboxes.
                                    {(mode.get() == Mode::Replace && !stops.blocked)
                                        .then(|| {
                                            view! {
                                                <div class="pt-1 space-y-1.5 border-t border-border">
                                                    {p
                                                        .rollback_delta
                                                        .map(|d| {
                                                            view! {
                                                                <label class="flex items-start gap-2">
                                                                    <Checkbox
                                                                        checked=Signal::derive(move || { confirm_rollback.get() })
                                                                        on_change=Callback::new(move |v: bool| {
                                                                            confirm_rollback.set(v);
                                                                        })
                                                                        attr:data-testid="confirm-rollback"
                                                                    />
                                                                    <span class="text-text-primary">
                                                                        {move || t!(i18n, unlock.ob_risk_rollback)} " ("
                                                                        {d.to_string()}")"
                                                                    </span>
                                                                </label>
                                                            }
                                                        })}
                                                    {(p.credentials_differ == Some(true))
                                                        .then(|| {
                                                            view! {
                                                                <label class="flex items-start gap-2">
                                                                    <Checkbox
                                                                        checked=Signal::derive(move || {
                                                                            confirm_credentials.get()
                                                                        })
                                                                        on_change=Callback::new(move |v: bool| {
                                                                            confirm_credentials.set(v);
                                                                        })
                                                                        attr:data-testid="confirm-credentials"
                                                                    />
                                                                    <span class="text-text-primary">
                                                                        {move || t!(i18n, unlock.ob_risk_credentials)}
                                                                    </span>
                                                                </label>
                                                            }
                                                        })}
                                                    {(p.target_state == TargetStateDto::Unreadable)
                                                        .then(|| {
                                                            view! {
                                                                <label class="flex items-start gap-2">
                                                                    <Checkbox
                                                                        checked=Signal::derive(move || { confirm_unverified.get() })
                                                                        on_change=Callback::new(move |v: bool| {
                                                                            confirm_unverified.set(v);
                                                                        })
                                                                        attr:data-testid="confirm-unverified"
                                                                    />
                                                                    <span class="text-text-primary">
                                                                        {move || t!(i18n, unlock.ob_risk_unverified)}
                                                                    </span>
                                                                </label>
                                                            }
                                                        })}
                                                </div>
                                            }
                                        })}
                                </div>
                            }
                        })
                }}

                // ---- Actions ----
                <div class="flex items-center justify-between gap-2 pt-1">
                    // Advanced ▸ Replace. Collapsed, and never the first thing offered.
                    <Button
                        variant=Variant::Ghost
                        size=Size::Sm
                        attr:data-testid="open-backup-advanced"
                        on:click=move |_: web_sys::MouseEvent| {
                            mode.update(|m| {
                                *m = if *m == Mode::Open { Mode::Replace } else { Mode::Open };
                            });
                            preview.set(None);
                            confirm_rollback.set(false);
                            confirm_credentials.set(false);
                            confirm_unverified.set(false);
                        }
                    >
                        {move || {
                            if mode.get() == Mode::Open {
                                t_string!(i18n, unlock.ob_advanced).to_owned()
                            } else {
                                t_string!(i18n, unlock.ob_advanced_back).to_owned()
                            }
                        }}
                    </Button>

                    <div class="flex items-center gap-2">
                        {move || {
                            let p = preview.get();
                            let is_replace = mode.get() == Mode::Replace;
                            match p {
                                None => {
                                    Either::Left(
                                        // Step 1 → 2.
                                        view! {
                                            <Button
                                                variant=Variant::Primary
                                                loading=busy.get()
                                                disabled=!can_preview.get()
                                                attr:data-testid="backup-preview-continue"
                                                on:click=move |_: web_sys::MouseEvent| load_preview()
                                            >
                                                {move || t!(i18n, unlock.ob_continue)}
                                            </Button>
                                        },
                                    )
                                }
                                Some(p) => {
                                    let stops = hard_stops(&p, mode.get());
                                    let unmet = is_replace
                                        && ((p.rollback_delta.is_some() && !confirm_rollback.get())
                                            || (p.credentials_differ == Some(true)
                                                && !confirm_credentials.get())
                                            || (p.target_state == TargetStateDto::Unreadable
                                                && !confirm_unverified.get()));
                                    Either::Right(
                                        // Step 2: act — unless a hard stop says otherwise.
                                        view! {
                                            <Button
                                                variant=if is_replace {
                                                    Variant::Danger
                                                } else {
                                                    Variant::Primary
                                                }
                                                loading=busy.get()
                                                disabled=stops.blocked || unmet
                                                attr:data-testid=if is_replace {
                                                    "replace-confirm"
                                                } else {
                                                    "open-backup-confirm"
                                                }
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    if is_replace { do_replace() } else { do_open() }
                                                }
                                            >
                                                {move || {
                                                    if is_replace {
                                                        t_string!(i18n, unlock.ob_replace_action).to_owned()
                                                    } else {
                                                        t_string!(i18n, unlock.ob_open_action).to_owned()
                                                    }
                                                }}
                                            </Button>
                                        },
                                    )
                                }
                            }
                        }}
                    </div>
                </div>
            </div>
        </Dialog>
    }
}

/// The refusals the backend will make, surfaced early. 🔴 **No checkbox gets past any of these**
/// — they are not risks to accept, they are answers of "no".
fn hard_stops(p: &BackupPreviewDto, mode: Mode) -> HardStops {
    let reason = if p.unknown_format {
        Some("format")
    } else if p.unknown_schema {
        Some("schema")
    } else if mode == Mode::Open && p.dest_occupied {
        // ① The constraint the whole design rests on. There is no override, and offering one
        // here would put 5.2's four dialogs straight back.
        Some("occupied")
    } else if mode == Mode::Replace && p.uuid_mismatch {
        Some("wrong_vault")
    } else if mode == Mode::Replace && p.target_state == TargetStateDto::Missing {
        // Nothing to replace — they wanted Open backup.
        Some("no_target")
    } else {
        None
    };
    HardStops {
        blocked: reason.is_some(),
        reason,
    }
}
