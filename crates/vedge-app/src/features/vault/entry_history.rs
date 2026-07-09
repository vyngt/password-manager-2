//! Version-history panel (slice 2.7). Lists the current version plus dated prior
//! snapshots with changed-field chips; reveals / copies a prior secret and
//! restores a prior version. Opened from the detail's "Version history" action
//! via a `target` signal (like [`FolderCustomize`](super::folder_customize)).
//!
//! Reveal reuses the sanctioned `get_history_value` path (backend-audited
//! `Viewed`); copy of a primary field routes through the backend clipboard
//! (`copy_history_field`, no plaintext to WASM); restore re-encrypts server-side
//! (`restore_history`, no plaintext out). Revealed values are short-lived — they
//! clear when another row is revealed or the panel closes.

use super::entry_view::{RelTime, long_date, relative_time, type_label_i18n};
use super::secret_display::SecretDisplay;
use crate::api;
use crate::features::settings::security_prefs::SecurityPrefsCtx;
use crate::features::vault::context::ActiveVault;
use crate::i18n::*;
use icondata as i;
use leptos::either::{Either, EitherOf3};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_i18n::I18nContext;
use leptos_icons::Icon;
use vedge_ipc::{EntryTypeDto, FieldSelectorDto, HistoryEntryDto, IndexEntryDto, PayloadDto};
use vedge_ui::components::Button;
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::feedback::{Dialog, DialogBody, DialogHeader, DialogTitle};
use vedge_ui::components::foundation::badge::Badge;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{
    BadgeSize, BadgeVariant, DialogSize, Size, ToastVariant, Variant,
};

/// The version-history dialog. Shown while `target` is `Some`; `on_restored`
/// fires after a successful restore so the page refreshes.
#[component]
pub fn EntryHistory(
    target: RwSignal<Option<IndexEntryDto>>,
    on_restored: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let sec = expect_context::<SecurityPrefsCtx>();
    let toast = use_toast();
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };
    let show_success = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, vault.dismiss).to_string());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Success)
                .dismiss_label(dismiss),
        );
    };

    let versions = RwSignal::new(Vec::<HistoryEntryDto>::new());
    let loading = RwSignal::new(false);
    // The single currently-revealed row: `(version, plaintext)`.
    let revealed = RwSignal::new(Option::<(u32, String)>::None);
    // Pending restore confirmation, holding the snapshot's `history_id`.
    let confirm = RwSignal::new(Option::<String>::None);

    // Load history each time the dialog opens; reset transient state.
    Effect::new(move |_| {
        let Some(entry) = target.get() else {
            return;
        };
        revealed.set(None);
        confirm.set(None);
        loading.set(true);
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry.id.clone();
        let err_prefix = untrack(|| t_string!(i18n, vault.err_history).to_string());
        spawn_local(async move {
            match api::entry::list_history(&vault_path, &id).await {
                Ok(rows) => versions.set(rows),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            loading.set(false);
        });
    });

    let close = Callback::new(move |()| {
        revealed.set(None);
        target.set(None);
    });

    // Reveal (or toggle off) a version's primary secret. `history_id = None`
    // reveals the live entry via `get_entry`; a snapshot via `get_history_value`.
    let do_reveal = move |version: u32, history_id: Option<String>| {
        if revealed.get_untracked().map(|(v, _)| v) == Some(version) {
            revealed.set(None);
            return;
        }
        let Some(entry) = target.get_untracked() else {
            return;
        };
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry.id.clone();
        let err_prefix = t_string!(i18n, vault.err_reveal).to_string();
        spawn_local(async move {
            let result = match history_id {
                Some(hid) => api::entry::get_history_value(&vault_path, &id, &hid).await,
                None => api::entry::get_entry(&vault_path, &id).await,
            };
            match result {
                Ok(payload) => {
                    if let Some(secret) = primary_secret(&payload) {
                        revealed.set(Some((version, secret)));
                    }
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    };

    // Copy a version's primary field to the OS clipboard (backend, auto-clear).
    let do_copy = move |history_id: Option<String>, field: FieldSelectorDto| {
        let Some(entry) = target.get_untracked() else {
            return;
        };
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry.id.clone();
        let secs = sec.0.get_untracked().clipboard_clear_seconds;
        let copied = t_string!(i18n, vault.copied).to_string();
        let err_prefix = t_string!(i18n, vault.err_copy).to_string();
        spawn_local(async move {
            let result = match history_id {
                Some(hid) => {
                    api::entry::copy_history_field(&vault_path, &id, &hid, &field, Some(secs)).await
                }
                None => api::entry::copy_field(&vault_path, &id, &field, Some(secs)).await,
            };
            match result {
                Ok(()) => show_success(copied),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    };

    // Run the confirmed restore, then close + refresh.
    let do_restore = move || {
        let (Some(hid), Some(entry)) = (confirm.get_untracked(), target.get_untracked()) else {
            return;
        };
        let vault_path = active.path.get_untracked().unwrap_or_default();
        let id = entry.id.clone();
        let err_prefix = t_string!(i18n, vault.err_history).to_string();
        confirm.set(None);
        spawn_local(async move {
            match api::entry::restore_history(&vault_path, &id, &hid).await {
                Ok(()) => {
                    revealed.set(None);
                    target.set(None);
                    on_restored.run(());
                }
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
        });
    };

    // Type-derived affordances (fixed for the whole entry).
    let entry_type = Signal::derive(move || target.get().map(|e| e.entry_type));
    let is_login = move || entry_type.get() == Some(EntryTypeDto::Login);
    let copy_field_of = move || entry_type.get().as_ref().and_then(primary_copy_field);
    let can_reveal = move || entry_type.get().as_ref().is_some_and(has_secret);

    let title = move || {
        if is_login() {
            t_string!(i18n, vault.password_history).to_string()
        } else {
            t_string!(i18n, vault.version_history).to_string()
        }
    };

    view! {
        <Dialog
            open=Signal::derive(move || target.get().is_some())
            on_close=close
            size=DialogSize::Md
            close_label=Signal::derive(move || t_string!(i18n, vault.close).to_string())
        >
            <DialogHeader>
                <DialogTitle>
                    <span class="flex items-center gap-2">
                        <span class="text-text-secondary">
                            <Icon attr:aria-hidden="true" icon=i::FaClockRotateLeftSolid />
                        </span>
                        <span>{title}</span>
                        {move || {
                            target
                                .get()
                                .map(|e| {
                                    view! {
                                        <span class="text-text-tertiary font-normal">
                                            {format!("· {}", e.name)}
                                        </span>
                                    }
                                })
                        }}
                        {move || {
                            entry_type
                                .get()
                                .map(|t| {
                                    let label = type_label_i18n(i18n, &t);
                                    view! {
                                        <Badge size=BadgeSize::Sm variant=BadgeVariant::Info>
                                            {label}
                                        </Badge>
                                    }
                                })
                        }}
                    </span>
                </DialogTitle>
            </DialogHeader>
            <DialogBody>
                <div class="flex flex-col gap-2 pb-4 min-w-[28rem]">
                    // Restore confirmation banner.
                    {move || {
                        confirm
                            .get()
                            .map(|_| {
                                view! {
                                    <div class="flex items-center justify-between gap-3 rounded-md border border-border bg-primary-muted px-3 py-2">
                                        <span class="text-sm text-text-secondary">
                                            {move || t!(i18n, vault.history_restore_confirm)}
                                        </span>
                                        <div class="flex items-center gap-2 shrink-0">
                                            <Button
                                                variant=Variant::Ghost
                                                size=Size::Sm
                                                on:click=move |_: web_sys::MouseEvent| confirm.set(None)
                                            >
                                                {move || t!(i18n, vault.cancel)}
                                            </Button>
                                            <Button
                                                variant=Variant::Primary
                                                size=Size::Sm
                                                on:click=move |_: web_sys::MouseEvent| do_restore()
                                            >
                                                {move || t!(i18n, vault.history_restore)}
                                            </Button>
                                        </div>
                                    </div>
                                }
                            })
                    }}
                    {move || {
                        if loading.get() {
                            EitherOf3::A(
                                view! {
                                    <p class="text-sm text-text-tertiary py-4 text-center">
                                        {move || t!(i18n, vault.loading)}
                                    </p>
                                },
                            )
                        } else if versions.get().is_empty() {
                            EitherOf3::B(
                                view! {
                                    <p class="text-sm text-text-tertiary py-4 text-center">
                                        {move || t!(i18n, vault.history_empty)}
                                    </p>
                                },
                            )
                        } else {
                            EitherOf3::C(
                                view! {
                                    <For
                                        each=move || versions.get()
                                        key=|v: &HistoryEntryDto| v.version
                                        let:v
                                    >
                                        {history_row(
                                            i18n,
                                            v,
                                            revealed,
                                            confirm,
                                            copy_field_of(),
                                            can_reveal(),
                                            do_reveal,
                                            do_copy,
                                        )}
                                    </For>
                                },
                            )
                        }
                    }}
                    <div class="flex items-start gap-1.5 pt-2 mt-1 border-t border-border text-xs text-text-tertiary">
                        <span class="mt-0.5">
                            <Icon attr:aria-hidden="true" icon=i::FaLockSolid />
                        </span>
                        <span>{move || t!(i18n, vault.history_kept_note)}</span>
                    </div>
                </div>
            </DialogBody>
        </Dialog>
    }
}

/// One version row: date + relative time, changed-field chips, and per-row
/// reveal / copy / restore actions. `copy_field` is `Some` when the type has a
/// backend-copyable primary field; `can_reveal` gates the eye action.
#[allow(clippy::too_many_arguments)]
fn history_row(
    i18n: I18nContext<Locale>,
    v: HistoryEntryDto,
    revealed: RwSignal<Option<(u32, String)>>,
    confirm: RwSignal<Option<String>>,
    copy_field: Option<FieldSelectorDto>,
    can_reveal: bool,
    do_reveal: impl Fn(u32, Option<String>) + Copy + 'static,
    do_copy: impl Fn(Option<String>, FieldSelectorDto) + Copy + 'static,
) -> impl IntoView {
    let version = v.version;
    let is_current = v.history_id.is_none();
    let history_id = v.history_id.clone();
    let changed = v.changed_fields.clone();
    // Cloned for the (reactive) date + relative-time label closures — i18n reads
    // live in `move ||`/`Signal::derive`, never eagerly (relocalizes on switch).
    let changed_at_date = v.changed_at.clone();
    let changed_at_rel = v.changed_at.clone();

    let reveal_hid = history_id.clone();
    let copy_hid = history_id.clone();

    let row_bg = if is_current { "bg-success-muted" } else { "" };
    let date_class = if is_current {
        "text-sm font-medium text-success-text"
    } else {
        "text-sm font-medium text-text-primary"
    };

    view! {
        <div class=format!(
            "flex flex-col gap-2 rounded-md px-3 py-2 border border-transparent {row_bg}",
        )>
            <div class="flex items-center gap-3">
                <div class="w-28 shrink-0">
                    // Date: "Current" for the live row, else the calendar date.
                    <div class=date_class>
                        {move || {
                            if is_current {
                                t_string!(i18n, vault.history_current).to_string()
                            } else {
                                long_date(&changed_at_date)
                            }
                        }}
                    </div>
                    <div class="text-xs text-text-tertiary">
                        {move || rel_label(i18n, &changed_at_rel)}
                    </div>
                </div>
                <div class="flex-1 flex flex-wrap gap-1">
                    {if changed.is_empty() {
                        Either::Left(
                            view! {
                                <Badge size=BadgeSize::Sm variant=BadgeVariant::Default>
                                    {move || t!(i18n, vault.history_created)}
                                </Badge>
                            },
                        )
                    } else {
                        Either::Right(
                            changed
                                .iter()
                                .map(|f| {
                                    let f = f.clone();
                                    view! {
                                        <Badge size=BadgeSize::Sm variant=BadgeVariant::Default>
                                            {move || changed_field_label(i18n, &f)}
                                        </Badge>
                                    }
                                })
                                .collect_view(),
                        )
                    }}
                </div>
                <div class="flex items-center gap-1 shrink-0 text-text-tertiary">
                    {can_reveal
                        .then(|| {
                            let hid = reveal_hid.clone();
                            view! {
                                <IconButton
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.history_reveal).to_string()
                                    })
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |_: web_sys::MouseEvent| {
                                        do_reveal(version, hid.clone());
                                    }
                                >
                                    {move || {
                                        if revealed.get().map(|(rv, _)| rv) == Some(version) {
                                            Either::Left(
                                                view! {
                                                    <Icon attr:aria-hidden="true" icon=i::FaEyeSlashSolid />
                                                },
                                            )
                                        } else {
                                            Either::Right(
                                                view! {
                                                    <Icon attr:aria-hidden="true" icon=i::FaEyeSolid />
                                                },
                                            )
                                        }
                                    }}
                                </IconButton>
                            }
                        })}
                    {copy_field
                        .map(|field| {
                            let hid = copy_hid.clone();
                            view! {
                                <IconButton
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, vault.history_copy).to_string()
                                    })
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |_: web_sys::MouseEvent| {
                                        do_copy(hid.clone(), field.clone());
                                    }
                                >
                                    <Icon attr:aria-hidden="true" icon=i::FaCopySolid />
                                </IconButton>
                            }
                        })}
                    {(!is_current)
                        .then(|| {
                            let hid = history_id.clone();
                            view! {
                                <Button
                                    variant=Variant::Ghost
                                    size=Size::Sm
                                    on:click=move |_: web_sys::MouseEvent| {
                                        confirm.set(hid.clone());
                                    }
                                >
                                    {move || t!(i18n, vault.history_restore)}
                                </Button>
                            }
                        })}
                </div>
            </div>
            // Revealed value (short-lived) for this row.
            {move || {
                revealed
                    .get()
                    .and_then(|(rv, val)| (rv == version).then_some(val))
                    .map(|val| {
                        view! {
                            <div class="pl-28">
                                <SecretDisplay value=Signal::derive(move || val.clone()) />
                            </div>
                        }
                    })
            }}
        </div>
    }
}

/// The relative-time sub-label for a row, localized.
fn rel_label(i18n: I18nContext<Locale>, changed_at: &str) -> String {
    let now_ms = chrono::Utc::now().timestamp_millis();
    match relative_time(changed_at, now_ms) {
        RelTime::JustNow => t_string!(i18n, vault.history_just_now).to_string(),
        RelTime::Today => t_string!(i18n, vault.history_today).to_string(),
        RelTime::Days(n) => format!("{n} {}", t_string!(i18n, vault.history_days_ago)),
        RelTime::Weeks(n) => format!("{n} {}", t_string!(i18n, vault.history_weeks_ago)),
        RelTime::Months(n) => format!("{n} {}", t_string!(i18n, vault.history_months_ago)),
    }
}

/// The version's primary revealable secret value (for the reveal display).
fn primary_secret(payload: &PayloadDto) -> Option<String> {
    match payload {
        PayloadDto::Login(p) => Some(p.password.clone()),
        PayloadDto::Card(p) => Some(p.number.clone()),
        PayloadDto::ApiKey(p) => Some(p.key.clone()),
        PayloadDto::SshKey(p) => Some(p.private_key_pem.clone()),
        PayloadDto::Note(p) => Some(p.content.clone()),
        PayloadDto::EnvVars(p) => p.vars.first().map(|v| v.value.clone()),
        PayloadDto::Identity(p) => p.national_id.clone(),
        PayloadDto::Document(_) | PayloadDto::Folder(_) => None,
    }
}

/// The primary field the backend clipboard can copy without a prior reveal.
/// `None` types still support reveal (and manual copy of the revealed value).
fn primary_copy_field(entry_type: &EntryTypeDto) -> Option<FieldSelectorDto> {
    match entry_type {
        EntryTypeDto::Login => Some(FieldSelectorDto::Password),
        EntryTypeDto::Card => Some(FieldSelectorDto::CardNumber),
        EntryTypeDto::ApiKey => Some(FieldSelectorDto::ApiKey),
        _ => None,
    }
}

/// Whether a type carries a revealable secret (gates the reveal action).
fn has_secret(entry_type: &EntryTypeDto) -> bool {
    matches!(
        entry_type,
        EntryTypeDto::Login
            | EntryTypeDto::Card
            | EntryTypeDto::ApiKey
            | EntryTypeDto::SshKey
            | EntryTypeDto::Note
            | EntryTypeDto::EnvVars
            | EntryTypeDto::Identity
    )
}

/// A human label for a changed-field key. Common content fields reuse existing
/// localized labels; the long tail is prettified from the raw key.
fn changed_field_label(i18n: I18nContext<Locale>, key: &str) -> String {
    match key {
        "name" => t_string!(i18n, vault.col_name).to_string(),
        "url" => t_string!(i18n, vault.col_url).to_string(),
        "password" => t_string!(i18n, vault.form_password).to_string(),
        "number" => t_string!(i18n, vault.field_card_number).to_string(),
        "cvv" => t_string!(i18n, vault.field_cvv).to_string(),
        "content" => t_string!(i18n, vault.field_content).to_string(),
        other => prettify_key(other),
    }
}

/// `card_number` → `Card number`. Fallback label for uncommon field keys.
fn prettify_key(key: &str) -> String {
    let spaced = key.replace('_', " ");
    let mut chars = spaced.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}
