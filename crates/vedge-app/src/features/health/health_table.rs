//! The health findings rendered as a `DataTable`. Columns: entry (name resolved
//! from `VaultIndex`, **zero decrypt**), field, issue (severity-tinted badge),
//! detail (score / reuse count / age + confidence), severity (badge). Clicking a
//! row asks the page to open that entry.
//!
//! Rows are pre-resolved into [`HealthRow`] (all-final strings) in a reactive
//! `Signal::derive` *before* they reach the table: a `DataTable` cell closure must
//! be `Send + Sync` and so cannot capture signals or call `t_string!` — the exact
//! pattern the audit table established.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos_i18n::I18nContext;
use vedge_ipc::{AgeConfidenceDto, FindingDto, FindingKindDto, SecretFieldDto, SeverityDto};
use vedge_ui::components::data_display::{
    CellValue, ColumnDef, ColumnType, ColumnWidth, DataTable, cell_fn, string_fn,
};
use vedge_ui::primitives::tokens::{Align, BadgeVariant};

use crate::i18n::{Locale, t_string, use_i18n};

/// A resolved table row. Every field is final (localized / formatted) so the
/// `Send + Sync` cell closures read plain data. `id` is the finding's index in
/// the report — stable and unique for a fixed report render.
#[derive(Clone, PartialEq)]
struct HealthRow {
    id: String,
    entry_id: String,
    entry_display: String,
    field_display: String,
    kind_label: String,
    kind_variant: BadgeVariant,
    detail: String,
    severity_label: String,
    severity_variant: BadgeVariant,
}

/// Badge tint for a finding's severity: High → Danger, Medium → Warning,
/// Low → Info.
fn severity_variant(severity: SeverityDto) -> BadgeVariant {
    match severity {
        SeverityDto::High => BadgeVariant::Danger,
        SeverityDto::Medium => BadgeVariant::Warning,
        SeverityDto::Low => BadgeVariant::Info,
    }
}

fn severity_label(i18n: I18nContext<Locale>, severity: SeverityDto) -> String {
    match severity {
        SeverityDto::High => t_string!(i18n, health.severity_high).to_owned(),
        SeverityDto::Medium => t_string!(i18n, health.severity_medium).to_owned(),
        SeverityDto::Low => t_string!(i18n, health.severity_low).to_owned(),
    }
}

/// Localized label for a secret field. `None` is an entry-level finding (Old).
/// `EnvVar` appends its variable **key** (non-secret); `LoginRecoveryCode` its
/// 1-based index — neither carries a secret value.
fn field_display(i18n: I18nContext<Locale>, field: Option<&SecretFieldDto>) -> String {
    let Some(field) = field else {
        return t_string!(i18n, health.field_none).to_owned();
    };
    match field {
        SecretFieldDto::LoginPassword => t_string!(i18n, health.field_login_password).to_owned(),
        SecretFieldDto::LoginRecoveryCode(index) => {
            format!(
                "{} #{}",
                t_string!(i18n, health.field_login_recovery_code),
                index.saturating_add(1)
            )
        }
        SecretFieldDto::CardNumber => t_string!(i18n, health.field_card_number).to_owned(),
        SecretFieldDto::CardCvv => t_string!(i18n, health.field_card_cvv).to_owned(),
        SecretFieldDto::CardPin => t_string!(i18n, health.field_card_pin).to_owned(),
        SecretFieldDto::SshPrivateKey => t_string!(i18n, health.field_ssh_private_key).to_owned(),
        SecretFieldDto::SshPassphrase => t_string!(i18n, health.field_ssh_passphrase).to_owned(),
        SecretFieldDto::ApiKeyKey => t_string!(i18n, health.field_api_key_key).to_owned(),
        SecretFieldDto::ApiKeySecret => t_string!(i18n, health.field_api_key_secret).to_owned(),
        SecretFieldDto::EnvVar(key) => {
            format!("{} {key}", t_string!(i18n, health.field_env_var))
        }
        SecretFieldDto::NoteContent => t_string!(i18n, health.field_note_content).to_owned(),
        SecretFieldDto::IdentityNationalId => {
            t_string!(i18n, health.field_identity_national_id).to_owned()
        }
    }
}

/// The issue label + its badge tint. The kind's inherent tint (weak → Danger,
/// reused → Warning, old → Info) reads alongside the per-finding severity badge.
fn kind_label_variant(i18n: I18nContext<Locale>, kind: &FindingKindDto) -> (String, BadgeVariant) {
    match kind {
        FindingKindDto::Weak { .. } => (
            t_string!(i18n, health.kind_weak).to_owned(),
            BadgeVariant::Danger,
        ),
        FindingKindDto::Reused { .. } => (
            t_string!(i18n, health.kind_reused).to_owned(),
            BadgeVariant::Warning,
        ),
        FindingKindDto::Old { .. } => (
            t_string!(i18n, health.kind_old).to_owned(),
            BadgeVariant::Info,
        ),
        FindingKindDto::Breached { .. } => (
            t_string!(i18n, health.kind_breached).to_owned(),
            BadgeVariant::Danger,
        ),
    }
}

/// Kind-specific detail: `score/4` for weak, `N entries` for reuse, and
/// `days · confidence` for age. Composed with `format!` + plain-key labels (no
/// interpolation macro — none is used in this codebase).
fn detail(i18n: I18nContext<Locale>, kind: &FindingKindDto) -> String {
    match kind {
        FindingKindDto::Weak { score, .. } => format!("{score}/4"),
        FindingKindDto::Reused { count, .. } => {
            format!("{count} {}", t_string!(i18n, health.reuse_unit))
        }
        FindingKindDto::Old {
            age_days,
            confidence,
        } => {
            let conf = match confidence {
                AgeConfidenceDto::Exact => t_string!(i18n, health.confidence_exact).to_owned(),
                AgeConfidenceDto::Estimated => {
                    t_string!(i18n, health.confidence_estimated).to_owned()
                }
            };
            format!("{age_days} {} · {conf}", t_string!(i18n, health.days_unit))
        }
        FindingKindDto::Breached { count } => {
            format!("{count} {}", t_string!(i18n, health.breached_unit))
        }
    }
}

/// Resolve an entry id to a display name (zero decrypt — the index is loaded).
/// A dangling id (entry hard-deleted since the scan) shows a muted label.
fn entry_display(
    i18n: I18nContext<Locale>,
    entry_id: &str,
    names: &HashMap<String, String>,
) -> String {
    names.get(entry_id).cloned().unwrap_or_else(|| {
        let short: String = entry_id.chars().take(8).collect();
        format!("{} {short}", t_string!(i18n, health.entry_deleted))
    })
}

fn build_row(
    i18n: I18nContext<Locale>,
    index: usize,
    finding: &FindingDto,
    names: &HashMap<String, String>,
) -> HealthRow {
    let (kind_label, kind_variant) = kind_label_variant(i18n, &finding.kind);
    HealthRow {
        id: index.to_string(),
        entry_id: finding.entry_id.clone(),
        entry_display: entry_display(i18n, &finding.entry_id, names),
        field_display: field_display(i18n, finding.field.as_ref()),
        kind_label,
        kind_variant,
        detail: detail(i18n, &finding.kind),
        severity_label: severity_label(i18n, finding.severity),
        severity_variant: severity_variant(finding.severity),
    }
}

#[component]
pub fn HealthTable(
    #[prop(into)] findings: Signal<Vec<FindingDto>>,
    #[prop(into)] names: Signal<HashMap<String, String>>,
    #[prop(into)] loading: Signal<bool>,
    /// Called with the entry id when a row is clicked (click-through to the
    /// entry in the vault view).
    #[prop(into)]
    on_select: Callback<String>,
) -> impl IntoView {
    let i18n = use_i18n();

    // Resolve rows in a reactive closure (relocalizes on language switch, and has
    // an owner for the `t_string!` reads inside `build_row`).
    let rows = Signal::derive(move || {
        let names = names.get();
        findings.with(|fs| {
            fs.iter()
                .enumerate()
                .map(|(i, f)| build_row(i18n, i, f, &names))
                .collect::<Vec<_>>()
        })
    });

    let columns = vec![
        ColumnDef {
            id: "entry",
            header: Signal::derive(move || t_string!(i18n, health.col_entry).to_owned()).into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|r: &HealthRow| CellValue::Text(r.entry_display.clone())),
        },
        ColumnDef {
            id: "field",
            header: Signal::derive(move || t_string!(i18n, health.col_field).to_owned()).into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Fixed(160),
            align: Align::Start,
            cell: cell_fn(|r: &HealthRow| CellValue::Text(r.field_display.clone())),
        },
        ColumnDef {
            id: "kind",
            header: Signal::derive(move || t_string!(i18n, health.col_kind).to_owned()).into(),
            col_type: ColumnType::Badge,
            sortable: false,
            width: ColumnWidth::Fixed(110),
            align: Align::Start,
            cell: cell_fn(|r: &HealthRow| CellValue::Badge {
                label: r.kind_label.clone(),
                variant: r.kind_variant,
            }),
        },
        ColumnDef {
            id: "detail",
            header: Signal::derive(move || t_string!(i18n, health.col_detail).to_owned()).into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Fixed(180),
            align: Align::Start,
            cell: cell_fn(|r: &HealthRow| CellValue::Text(r.detail.clone())),
        },
        ColumnDef {
            id: "severity",
            header: Signal::derive(move || t_string!(i18n, health.col_severity).to_owned()).into(),
            col_type: ColumnType::Badge,
            sortable: false,
            width: ColumnWidth::Fixed(110),
            align: Align::End,
            cell: cell_fn(|r: &HealthRow| CellValue::Badge {
                label: r.severity_label.clone(),
                variant: r.severity_variant,
            }),
        },
    ];

    view! {
        <div data-testid="health-table">
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|r: &HealthRow| r.id.clone())
                loading=loading
                empty_message=Signal::derive(move || t_string!(i18n, health.table_empty).to_owned())
                on_row_click=Callback::new(move |r: HealthRow| on_select.run(r.entry_id))
            />
        </div>
    }
}
