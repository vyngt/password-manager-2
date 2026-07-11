//! Audit-log read command (slice 4.1).
//!
//! See `commands/vault.rs` for the rationale behind the `#![allow]` below:
//! macro expansion of `#[tauri::command(rename_all = "snake_case")]` trips
//! `unreachable` / `let_underscore_must_use`, and holding the session mutex
//! across the command body trips `significant_drop_tightening`.

#![allow(
    clippy::unreachable,
    clippy::let_underscore_must_use,
    clippy::significant_drop_tightening
)]

use std::path::PathBuf;

use tracing::instrument;

use vedge_core::domain::shared::VaultId;
use vedge_core::list_audit as list_audit_core;

use crate::dto::audit::{AuditPageDto, AuditQueryDto, audit_event_to_dto, audit_query_from_dto};
use crate::error::CommandError;
use crate::state::AppState;

fn vault_id_from_string(s: &str) -> VaultId {
    VaultId::new(PathBuf::from(s))
}

/// List the audit trail — **audit-silent** (writes no rows, bumps no
/// `accessed_at`).
///
/// Filters (action / entry / date range) are applied SQL-side; the returned
/// page carries the total match count for the "showing X–Y of N" summary. Entry
/// names are resolved by the app from `VaultIndex` — no decrypt.
#[tauri::command(rename_all = "snake_case")]
#[instrument(skip_all, fields(vault_path = %vault_path))]
pub async fn list_audit(
    vault_path: String,
    query: AuditQueryDto,
    state: tauri::State<'_, AppState>,
) -> Result<AuditPageDto, CommandError> {
    let vault_id = vault_id_from_string(&vault_path);
    let handle = state.get_session(&vault_id)?;
    let guard = handle.lock().await;

    let domain_query = audit_query_from_dto(&query)?;
    let page = list_audit_core(&guard, domain_query).await?;
    Ok(AuditPageDto {
        events: page.events.iter().map(audit_event_to_dto).collect(),
        total: page.total,
    })
}
