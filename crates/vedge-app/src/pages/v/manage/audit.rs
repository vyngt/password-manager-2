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
use vedge_ui::components::Button;
use vedge_ui::components::data_display::pagination::{Pagination, PaginationModel};
use vedge_ui::components::feedback::toast::provider::use_toast;
use vedge_ui::components::feedback::toast::types::ToastInput;
use vedge_ui::components::form::date_picker::{DatePicker, DatePickerValue};
use vedge_ui::components::select::{Select, SelectItem};
use vedge_ui::primitives::tokens::{Size, ToastVariant, Variant};

use crate::api;
use crate::features::audit::audit_table::{AuditTable, action_label};
use crate::features::audit::filters::{AUDIT_PAGE_SIZE, AuditView};
use crate::features::date_i18n::{calendar_labels, locale_tag};
use crate::features::vault::context::ActiveVault;
use crate::i18n::{t, t_string, use_i18n};

/// The `AuditAction` names in core-enum order — **derived** from the shared
/// wire-format table (`vedge_ipc::ACTION_NAMES`) so this dropdown can never drift
/// from the backend: a new core variant grows `ACTION_NAMES`, which changes this
/// array's length and fails to compile here until it's updated. Each name is
/// localized via [`action_label`]; an unrecognized one falls through to its raw
/// form there.
const AUDIT_ACTIONS: [&str; 23] = vedge_ipc::ACTION_NAMES;

#[component]
pub fn AuditPage() -> impl IntoView {
    let i18n = use_i18n();
    let active = expect_context::<ActiveVault>();
    let toast = use_toast();
    // Locale tag + panel aria-labels for the two date filters, so the calendar
    // renders Vietnamese under `vi` and its nav is localized (slice 4.9a P1).
    let cal_locale = locale_tag(i18n);
    let cal_labels = calendar_labels(i18n);
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
            // `try_get_value` (not `get_value`): the page may have unmounted
            // mid-flight (sidebar nav, idle auto-lock, or OS screen lock), disposing
            // this token — `get_value` would panic on the disposed value. Mirrors
            // the health page's scan guard (`health.rs`).
            if req_gen.try_get_value() != Some(token) {
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
    // Any facet or offset set → the clear-filters affordance appears. `AuditView`
    // derives `PartialEq`, so this is a plain struct comparison against default.
    let is_dirty = Signal::derive(move || view_state.get() != AuditView::default());

    view! {
        // Flex column that fills the `/v` outlet and never scrolls itself: the
        // header/filter/summary bands and the pagination footer stay fixed, and
        // the `AuditTable` band (`flex-1 min-h-0`) owns the remaining height so
        // the table's own `.data-table-wrapper` becomes the scroll container and
        // its sticky `<thead>` activates — no new CSS (see slice 4.9a P2).
        <div class="h-full p-6 flex flex-col" data-testid="audit-page">
            <div class="max-w-5xl w-full mx-auto flex-1 min-h-0 flex flex-col gap-4">
                <h1 class="text-xl font-semibold text-text-primary">
                    {move || t_string!(i18n, audit.title).to_owned()}
                </h1>

                // ---- Filter bar ----
                <div class="flex flex-wrap items-end gap-3">
                    <div class="flex flex-col gap-1" data-testid="audit-filter-action">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_action).to_owned()}
                        </span>
                        // `options` is a reactive `Signal` so a locale switch
                        // relocalizes the labels in place — the Select keeps its
                        // open/highlight/type-ahead state (no remount wrapper).
                        <Select
                            options=Signal::derive(move || {
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
                                options
                            })
                            value=Signal::derive(move || {
                                view_state.with(|v| v.filters.action.clone().unwrap_or_default())
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
                    </div>

                    <div class="flex flex-col gap-1" data-testid="audit-filter-entry">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_entry).to_owned()}
                        </span>
                        // Reactive `options`: relocalizes the "all entries" label and
                        // rebuilds when the entry index changes — without remounting.
                        <Select
                            options=Signal::derive(move || {
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
                                options
                            })
                            value=Signal::derive(move || {
                                view_state.with(|v| v.filters.entry_id.clone().unwrap_or_default())
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
                    </div>

                    <div class="flex flex-col gap-1" data-testid="audit-filter-since">
                        <span class="text-xs text-text-secondary">
                            {move || t_string!(i18n, audit.filter_since).to_owned()}
                        </span>
                        <DatePicker
                            id="audit-since"
                            locale=cal_locale
                            dialog_label=cal_labels.dialog
                            prev_month_label=cal_labels.prev_month
                            next_month_label=cal_labels.next_month
                            prev_year_label=cal_labels.prev_year
                            next_year_label=cal_labels.next_year
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
                            locale=cal_locale
                            dialog_label=cal_labels.dialog
                            prev_month_label=cal_labels.prev_month
                            next_month_label=cal_labels.next_month
                            prev_year_label=cal_labels.prev_year
                            next_year_label=cal_labels.next_year
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

                    // Clear-filters — shown only when a filter is active (`Button`'s
                    // `disabled` is not reactive, so hide rather than disable). One
                    // struct assignment resets every facet AND the page offset.
                    <Show when=move || is_dirty.get()>
                        <Button
                            variant=Variant::Secondary
                            size=Size::Sm
                            class="whitespace-nowrap"
                            attr:data-testid="audit-clear-filters"
                            on:click=move |_| view_state.set(AuditView::default())
                        >
                            {move || t!(i18n, audit.clear_filters)}
                        </Button>
                    </Show>
                </div>

                // Localized result-count summary — the page owns it. Pagination's
                // built-in summary is English-only (its localizable `summary`
                // override was dead API surface and was removed in slice 4.9a P5),
                // so the localized count lives here as a fixed band above the table.
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
