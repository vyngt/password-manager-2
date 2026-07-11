//! Audit-log read wrapper. Mirrors `vedge-tauri/src/commands/audit.rs`.

use serde::Serialize;

use vedge_ipc::{AuditPageDto, AuditQueryDto};

use crate::api::call::call;
use crate::api::error::ApiError;

/// Fetch one filtered, paged slice of the audit trail. Audit-silent server-side.
pub async fn list_audit(vault_path: &str, query: &AuditQueryDto) -> Result<AuditPageDto, ApiError> {
    #[derive(Serialize)]
    struct Args<'a> {
        vault_path: &'a str,
        query: &'a AuditQueryDto,
    }
    call("list_audit", &Args { vault_path, query }).await
}
