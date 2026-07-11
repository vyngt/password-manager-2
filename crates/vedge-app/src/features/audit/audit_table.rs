//! The audit trail rendered as a `DataTable` — the design system's first
//! production use of it. Columns: action (severity badge), entry (name resolved
//! from `VaultIndex`, **zero decrypt**), time (absolute date + `HH:MM:SS`).
//!
//! Rows are pre-resolved into [`AuditRow`] (all-final strings) in a reactive
//! `Signal::derive` *before* they reach the table: a `DataTable` cell closure
//! must be `Send + Sync` and so cannot capture signals or call `t_string!`.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos_i18n::I18nContext;
use vedge_ipc::{AuditEventDto, AuditPageDto};
use vedge_ui::components::data_display::{
    CellValue, ColumnDef, ColumnType, ColumnWidth, DataTable, cell_fn, string_fn,
};
use vedge_ui::primitives::tokens::{Align, BadgeVariant};

use crate::features::vault::entry_view::{clock_time, long_date};
use crate::i18n::{Locale, t_string, use_i18n};

/// A resolved table row. Every field is final (localized / formatted) so the
/// `Send + Sync` cell closures read plain data.
#[derive(Clone, PartialEq)]
struct AuditRow {
    id: String,
    action_label: String,
    severity: BadgeVariant,
    entry_display: String,
    time_display: String,
}

/// Badge severity by action, per the 4.1 spec's severity table: `Danger` for
/// destructive/recovery (permanent-delete, recovery-used), `Warning` for
/// sensitive mutations (delete, password-change, export), `Info` for session
/// lifecycle (unlock / biometric-unlock / lock), and `Default` for everything
/// else (create, update, restore, tag ops, …).
#[must_use]
pub fn severity(action: &str) -> BadgeVariant {
    match action {
        "PermanentlyDeleted" | "RecoveryUsed" => BadgeVariant::Danger,
        "Deleted" | "PasswordChanged" | "Exported" => BadgeVariant::Warning,
        "Unlocked" | "BiometricUnlocked" | "Locked" => BadgeVariant::Info,
        _ => BadgeVariant::Default,
    }
}

/// Localized label for a raw `PascalCase` action name. An action this build
/// does not recognize (a variant added by a later slice, met on a downgrade)
/// falls through to its raw name — the forward-compat "muted raw chip" path.
#[must_use]
pub fn action_label(i18n: I18nContext<Locale>, action: &str) -> String {
    match action {
        "Unlocked" => t_string!(i18n, audit.action_unlocked).to_owned(),
        "Locked" => t_string!(i18n, audit.action_locked).to_owned(),
        "Created" => t_string!(i18n, audit.action_created).to_owned(),
        "Viewed" => t_string!(i18n, audit.action_viewed).to_owned(),
        "Updated" => t_string!(i18n, audit.action_updated).to_owned(),
        "Deleted" => t_string!(i18n, audit.action_deleted).to_owned(),
        "Restored" => t_string!(i18n, audit.action_restored).to_owned(),
        "PermanentlyDeleted" => t_string!(i18n, audit.action_permanently_deleted).to_owned(),
        "Exported" => t_string!(i18n, audit.action_exported).to_owned(),
        "PasswordChanged" => t_string!(i18n, audit.action_password_changed).to_owned(),
        "TagCreated" => t_string!(i18n, audit.action_tag_created).to_owned(),
        "TagRenamed" => t_string!(i18n, audit.action_tag_renamed).to_owned(),
        "TagDeleted" => t_string!(i18n, audit.action_tag_deleted).to_owned(),
        "RecoveryUsed" => t_string!(i18n, audit.action_recovery_used).to_owned(),
        "BiometricUnlocked" => t_string!(i18n, audit.action_biometric_unlocked).to_owned(),
        other => other.to_owned(),
    }
}

fn entry_display(
    i18n: I18nContext<Locale>,
    event: &AuditEventDto,
    names: &HashMap<String, String>,
) -> String {
    match &event.entry_id {
        // Vault-level event (unlock, password change, …): no entry.
        None => t_string!(i18n, audit.entry_vault).to_owned(),
        Some(id) => names.get(id).cloned().unwrap_or_else(|| {
            // Dangling id: the entry was hard-deleted. Show a muted label + short id.
            let short: String = id.chars().take(8).collect();
            format!(
                "{} {short}",
                t_string!(i18n, audit.entry_deleted).to_owned()
            )
        }),
    }
}

fn build_row(
    i18n: I18nContext<Locale>,
    event: &AuditEventDto,
    names: &HashMap<String, String>,
) -> AuditRow {
    AuditRow {
        id: event.id.clone(),
        action_label: action_label(i18n, &event.action),
        severity: severity(&event.action),
        entry_display: entry_display(i18n, event, names),
        time_display: format!(
            "{} {}",
            long_date(&event.occurred_at),
            clock_time(&event.occurred_at)
        ),
    }
}

#[component]
pub fn AuditTable(
    #[prop(into)] page: Signal<AuditPageDto>,
    #[prop(into)] names: Signal<HashMap<String, String>>,
    #[prop(into)] loading: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();

    // Resolve rows in a reactive closure (relocalizes on language switch, and
    // has an owner for the `t_string!` reads inside `build_row`).
    let rows = Signal::derive(move || {
        let names = names.get();
        page.with(|p| {
            p.events
                .iter()
                .map(|e| build_row(i18n, e, &names))
                .collect::<Vec<_>>()
        })
    });

    // Audit rows are immutable, so the ULID id is a safe stable key — this is the
    // one list in the app that does NOT need `vault_table`'s composite-tuple key.
    let columns = vec![
        ColumnDef {
            id: "action",
            header: Signal::derive(move || t_string!(i18n, audit.col_action).to_owned()).into(),
            col_type: ColumnType::Badge,
            sortable: false,
            width: ColumnWidth::Fixed(170),
            align: Align::Start,
            cell: cell_fn(|r: &AuditRow| CellValue::Badge {
                label: r.action_label.clone(),
                variant: r.severity,
            }),
        },
        ColumnDef {
            id: "entry",
            header: Signal::derive(move || t_string!(i18n, audit.col_entry).to_owned()).into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|r: &AuditRow| CellValue::Text(r.entry_display.clone())),
        },
        ColumnDef {
            id: "time",
            header: Signal::derive(move || t_string!(i18n, audit.col_time).to_owned()).into(),
            col_type: ColumnType::Date,
            sortable: false,
            width: ColumnWidth::Fixed(210),
            align: Align::End,
            cell: cell_fn(|r: &AuditRow| CellValue::Text(r.time_display.clone())),
        },
    ];

    view! {
        <div data-testid="audit-table">
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|r: &AuditRow| r.id.clone())
                loading=loading
                empty_message=Signal::derive(move || t_string!(i18n, audit.empty).to_owned())
            />
        </div>
    }
}
