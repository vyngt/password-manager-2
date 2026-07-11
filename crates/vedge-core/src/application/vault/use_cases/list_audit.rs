//! List the audit trail.
//!
//! **Audit-silent by construction.** This use case reads `audit_log` and nothing
//! else: it decrypts no payload, writes no [`AuditAction::Viewed`], and does not
//! touch `accessed_at`. Auditing the act of reading the audit log would be
//! self-referential log spam. This is the first deliberate audit-silent read in
//! the codebase — later health-scan work cites this precedent.
//!
//! Takes `&VaultSession` (not `&VaultIndex`): the trail lives in the repo, not
//! the index. Mirrors [`list_history`](super::list_history), which is likewise
//! deliberately unaudited.
//!
//! [`AuditAction::Viewed`]: crate::domain::vault::entities::AuditAction::Viewed

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::vault::entities::{AuditPage, AuditQuery};
use crate::domain::vault::errors::VaultError;

/// Upper bound on a single page — a filter query never returns more than this
/// many rows, regardless of the requested `limit`.
pub const AUDIT_PAGE_MAX: u32 = 200;

/// Default page size when the caller does not specify one.
pub const AUDIT_PAGE_DEFAULT: u32 = 50;

/// Read one filtered, paged slice of the audit trail. The `limit` is clamped to
/// `1..=AUDIT_PAGE_MAX`; every predicate is applied in SQL by the repository.
#[instrument(skip_all, fields(entry_id = ?input.entry_id, actions = input.actions.len()))]
pub async fn list_audit(
    session: &VaultSession,
    mut input: AuditQuery,
) -> Result<AuditPage, VaultError> {
    input.limit = input.limit.clamp(1, AUDIT_PAGE_MAX);
    session.repo.query_audit(&input).await
}
