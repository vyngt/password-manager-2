//! `lock_vault` — record the lock event and consume the session.
//!
//! Appending `AuditAction::Locked` must happen **before** the session
//! drops: once the session is gone the `repo` handle goes with it (held
//! inside `VaultSession`). We pull the `Arc` out, audit, then let the
//! session value drop.
//!
//! [`record_lock`] is the audit half factored out of [`lock_vault`] so a caller
//! that only holds a `&VaultSession` — a shared session behind an `Arc<Mutex<…>>`
//! that it cannot move out (the backend session registry, the TTL reaper, and the
//! screen-lock watcher, slice 4.5) — can land the `Locked` row and then let the
//! last `Arc` clone drop to zeroize. This retires the old `Arc::try_unwrap` × 256
//! spin, which timed out under contention and **silently skipped the audit row**.

use tracing::instrument;

use crate::application::vault::session::VaultSession;
use crate::domain::shared::now;
use crate::domain::vault::entities::{AuditAction, AuditEvent};
use crate::domain::vault::errors::VaultError;

/// Append the `AuditAction::Locked` row for a session **without** consuming it.
///
/// Clones the `repo` `Arc` (so the write outlives the session guard the caller
/// holds) and appends. The caller is responsible for dropping the session
/// afterward — `VaultSession::Drop` zeroizes the KEK and index when the last
/// `Arc` clone releases.
#[instrument(skip_all, fields(vault_id = %session.vault_id()))]
pub async fn record_lock(session: &VaultSession) -> Result<(), VaultError> {
    let repo = std::sync::Arc::clone(&session.repo);

    let event = AuditEvent {
        id: ulid::Ulid::new().to_string(),
        entry_id: None,
        action: AuditAction::Locked,
        occurred_at: now(),
        device_id: None,
    };
    repo.append_audit(&event).await
}

/// Close the session's DB pool, releasing the OS file handle SYNCHRONOUSLY (slice 5.2.1).
///
/// 🔴 Call this on an **interactive** lock — the one the user triggers before a file op
/// (an in-place `revert_to_snapshot`, a `.vbk` restore) might follow. sqlx's pool close is
/// async on `Drop`, so without this the `.vdb` handle lingers past the lock and the following
/// swap races it on Windows (`os error 5`). Closing here makes the handle GONE by the time the
/// lock returns — deterministic, not a retry.
///
/// Deliberately NOT part of [`record_lock`]: the TTL reaper and the screen-lock watcher lock a
/// session and then READ its audit trail, and nothing reverts right after those, so they must
/// keep the connection alive.
pub async fn close_session_db(session: &VaultSession) {
    session.repo.close().await;
}

pub async fn lock_vault(session: VaultSession) -> Result<(), VaultError> {
    let audit_result = record_lock(&session).await;

    // Drop the session regardless of audit outcome — Drop zeroizes the KEK
    // and the VaultIndex. Propagate any audit error to the caller after.
    drop(session);
    audit_result
}
