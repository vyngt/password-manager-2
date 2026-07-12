//! Audit page (`/v/audit`) — browse + filter the trail the core has written
//! since Phase 1. Filters (action / entry / date range) are applied SQL-side by
//! the core; entry names are resolved from the in-memory index with **zero
//! decryption**. Data loading follows the house `RwSignal` + refresh +
//! `spawn_local` + `Effect` idiom (see `manage/vault.rs`), not `Resource`/`Action`.

use std::collections::HashMap;

use leptos::either::Either;
use leptos::prelude::*;
use leptos::task::spawn_local;
use vedge_ipc::{AuditPageDto, IndexEntryDto};
use vedge_ui::components::data_display::pagination::{Pagination, PaginationModel};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::date_picker::{DatePicker, DatePickerValue};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::ToastVariant;

use crate::api;
use crate::features::audit::audit_table::{AuditTable, action_label};
use crate::features::audit::filters::{AUDIT_PAGE_SIZE, AuditView};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

/// The 17 `AuditAction` names in the same order as the core enum. Used to build
/// the action-filter dropdown; each is localized via [`action_label`].
const AUDIT_ACTIONS: [&str; 17] = [
    "Unlocked",
    "Locked",
    "Created",
    "Viewed",
    "Updated",
    "Deleted",
    "Restored",
    "PermanentlyDeleted",
    "Exported",
    "PasswordChanged",
    "TagCreated",
    "TagRenamed",
    "TagDeleted",
    "RecoveryUsed",
    "BiometricUnlocked",
    "TotpRevealed",
    "HealthScanned",
];

#[component]
pub fn AuditPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    // Dismiss label read `untrack`ed so this is safe from `spawn_local` futures.
    let show_error = move |msg: String| {
        let dismiss = untrack(|| t_string!(i18n, audit.dismiss).to_owned());
        toast.show(
            ToastInput::new(msg)
                .variant(ToastVariant::Danger)
                .dismiss_label(dismiss),
        );
    };

    let view_state = RwSignal::new(AuditView::default());
    let page_data = RwSignal::new(AuditPageDto {
        events: Vec::new(),
        total: 0,
    });
    let loading = RwSignal::new(false);
    let index = RwSignal::new(Vec::<IndexEntryDto>::new());
    // Generation token: only the newest in-flight fetch may apply its result, so
    // an out-of-order (slow) response can't overwrite a fresher page.
    let req_gen = StoredValue::new(0_u32);

    // id -> name, for zero-decrypt entry-name resolution + the entry filter.
    // A `Memo` (not `Signal::derive`) so the map is rebuilt only when `index`
    // changes — not on every audit-page fetch / filter toggle that reads it.
    let names = Memo::new(move |_| {
        index
            .get()
            .into_iter()
            .map(|e| (e.id, e.name))
            .collect::<HashMap<String, String>>()
    });

    // Load the entry index once per active vault. Non-fatal on failure (the entry
    // column then falls back to ids).
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

    // Fetch the audit page whenever the vault, filters, or offset change.
    let refresh = move || {
        let path = untrack(|| active.path.get()).unwrap_or_default();
        if path.is_empty() {
            page_data.set(AuditPageDto {
                events: Vec::new(),
                total: 0,
            });
            return;
        }
        let query = untrack(|| view_state.get()).to_query();
        let err_prefix = untrack(|| t_string!(i18n, audit.err_load).to_owned());
        let token = req_gen.get_value().wrapping_add(1);
        req_gen.set_value(token);
        loading.set(true);
        spawn_local(async move {
            let result = api::audit::list_audit(&path, &query).await;
            // A newer request superseded this one → drop the stale response so it
            // can't overwrite a fresher page (or latch `loading` off early).
            if req_gen.get_value() != token {
                return;
            }
            match result {
                Ok(p) => page_data.set(p),
                Err(e) => show_error(format!("{err_prefix}{e}")),
            }
            loading.set(false);
        });
    };
    Effect::new(move |_| {
        let _ = active.path.get();
        let _ = view_state.get();
        refresh();
    });

    // Pagination derived state (1-indexed page ⇄ offset).
    let page_num = Signal::derive(move || view_state.with(|v| v.offset / AUDIT_PAGE_SIZE + 1));
    let total_pages = Signal::derive(move || {
        let total = page_data.with(|p| p.total);
        (total.div_ceil(u64::from(AUDIT_PAGE_SIZE)) as u32).max(1)
    });

    view! {
        <div class="h-full overflow-y-auto p-6" data-testid="audit-page">
            <div class="max-w-5xl mx-auto space-y-4">
                <h1 class="text-xl font-semibold text-text-primary">
                    {move || t_string!(i18n, audit.title).to_owned()}
                </h1>

                // ---- Filter bar ----
                <div class="flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1" data-testid="audit-filter-action">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_action).to_owned()}
                        </span>
                        {move || {
                            let mut options = vec![
                                SelectItem::option(
                                    "",
                                    t_string!(i18n, audit.filter_all_actions).to_owned(),
                                ),
                            ];
                            options
                                .extend(
                                    AUDIT_ACTIONS
                                        .iter()
                                        .map(|a| SelectItem::option(*a, action_label(i18n, a))),
                                );
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        view_state
                                            .with(|v| v.filters.action.clone().unwrap_or_default())
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, audit.filter_action).to_owned()
                                    })
                                    on_change=Callback::new(move |val: String| {
                                        view_state
                                            .update(|v| {
                                                v.set_action(if val.is_empty() { None } else { Some(val) });
                                            });
                                    })
                                />
                            }
                        }}
                    </div>

                    <div class="flex flex-col gap-1" data-testid="audit-filter-entry">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_entry).to_owned()}
                        </span>
                        {move || {
                            let mut options = vec![
                                SelectItem::option(
                                    "",
                                    t_string!(i18n, audit.filter_all_entries).to_owned(),
                                ),
                            ];
                            options
                                .extend(
                                    index
                                        .get()
                                        .into_iter()
                                        .map(|e| SelectItem::option(e.id, e.name)),
                                );
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || {
                                        view_state
                                            .with(|v| v.filters.entry_id.clone().unwrap_or_default())
                                    })
                                    aria_label=Signal::derive(move || {
                                        t_string!(i18n, audit.filter_entry).to_owned()
                                    })
                                    on_change=Callback::new(move |val: String| {
                                        view_state
                                            .update(|v| {
                                                v.set_entry(if val.is_empty() { None } else { Some(val) });
                                            });
                                    })
                                />
                            }
                        }}
                    </div>

                    <div class="flex flex-col gap-1" data-testid="audit-filter-since">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_since).to_owned()}
                        </span>
                        <DatePicker
                            id="audit-since"
                            value=Signal::derive(move || {
                                DatePickerValue::Single(view_state.with(|v| v.filters.since))
                            })
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, audit.filter_any_date).to_owned()
                            })
                            on_change=Callback::new(move |val: DatePickerValue| {
                                view_state.update(|v| v.set_since(val.as_single()));
                            })
                        />
                    </div>

                    <div class="flex flex-col gap-1" data-testid="audit-filter-until">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_until).to_owned()}
                        </span>
                        <DatePicker
                            id="audit-until"
                            value=Signal::derive(move || {
                                DatePickerValue::Single(view_state.with(|v| v.filters.until))
                            })
                            placeholder=Signal::derive(move || {
                                t_string!(i18n, audit.filter_any_date).to_owned()
                            })
                            on_change=Callback::new(move |val: DatePickerValue| {
                                view_state.update(|v| v.set_until(val.as_single()));
                            })
                        />
                    </div>
                </div>

                // ---- Localized summary (Pagination's own summary is hardcoded English) ----
                <p class="text-xs text-text-tertiary" data-testid="audit-summary">
                    {move || {
                        let total = page_data.with(|p| p.total);
                        if total == 0 {
                            Either::Left(view! { {move || t!(i18n, audit.summary_empty)} })
                        } else {
                            let offset = u64::from(view_state.with(|v| v.offset));
                            let count = page_data.with(|p| p.events.len()) as u64;
                            let start = offset + 1;
                            let end = offset + count;
                            Either::Right(
                                view! {
                                    {t!(
                                        i18n, audit.summary, start = start, end = end, total = total
                                    )}
                                },
                            )
                        }
                    }}
                </p>

                <AuditTable page=page_data names=names loading=loading />

                // ---- Pagination (hidden for a single page) ----
                {move || {
                    if total_pages.get() <= 1 {
                        ().into_any()
                    } else {
                        view! {
                            <div data-testid="audit-pagination">
                                <Pagination
                                    model=PaginationModel::Offset
                                    page=page_num
                                    total_pages=total_pages
                                    prev_label=Signal::derive(move || {
                                        t_string!(i18n, audit.prev).to_owned()
                                    })
                                    next_label=Signal::derive(move || {
                                        t_string!(i18n, audit.next).to_owned()
                                    })
                                    nav_label=Signal::derive(move || {
                                        t_string!(i18n, audit.pagination_nav).to_owned()
                                    })
                                    on_page_change=Callback::new(move |p: u32| {
                                        view_state.update(|v| v.offset = (p - 1) * AUDIT_PAGE_SIZE);
                                    })
                                />
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}
