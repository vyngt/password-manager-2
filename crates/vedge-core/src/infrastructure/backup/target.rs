//! Read a target vault's plaintext identity WITHOUT unlocking it (slice 5.2b; three-state
//! since 5.2.2).
//!
//! `replace_vault_from_backup` and `inspect_backup` must compare a backup's `vault_uuid` /
//! `schema_version` against the vault they are about to overwrite. Those are ordinary
//! (unencrypted) `vault_config` columns, so a read-only `SELECT` — no KEK, no migration —
//! is enough.
//!
//! ## 🔴 Finding H2 — and why it is fixed HERE, not at the call site
//!
//! 5.2b returned `Option<TargetIdentity>` and so collapsed two very different facts into
//! one `None`: *"there is no vault here"* and *"there is a vault here but I could not read
//! it."* Restore then wrapped every identity check in `if let Some(target_id)`, which meant
//! **an unreadable target silently disarmed the uuid-mismatch refusal** — the single guard
//! standing between a user and restoring the wrong vault over their real one.
//!
//! [`TargetState`] closes it: the three cases are distinguishable, so a caller *cannot*
//! fall through the unreadable one by accident — it has to say, explicitly, what it does
//! about a target it could not identify.
//!
//! ### The dirty-`-wal` theory, and why there is no `mode=rw` fallback
//!
//! The 5.2.2 spec proposed a second fix: most "unreadable" targets are supposedly not
//! corrupt at all, just carrying a dirty `-wal` that a `mode=ro` connection cannot replay
//! (it can't create the `-shm`), so we should fall back to `mode=rw`.
//!
//! **Measured, and it does not reproduce.** Against a real crash image (a 300 KB
//! uncheckpointed `-wal`, no `-shm`, no live handle), `mode=ro` connects and reads the
//! replayed rows — it never needed the `-shm` on disk. The fallback would therefore be a
//! branch that never runs, and on the one path where it *could* run it would **write**
//! (replaying a WAL, materialising an `-shm`) to a vault the user has not unlocked and may
//! be about to discard — contradicting this module's own reason for being read-only, on a
//! probe that runs against **every recent vault at startup**. An untested write path in a
//! startup probe is a worse trade than the theoretical robustness it buys, so it is not
//! here. `a_dirty_wal_target_stays_readable_and_identified` (in `tests/backup_restore.rs`)
//! pins the property that actually matters: a crashed vault stays **identified**, so the
//! uuid guard stays **armed**.
//!
//! 🔴 **And never `mode=rwc`.** That *creates* the file, fabricating a phantom empty vault
//! at a path that has none — a bug 5.2b already had to fix once, at the repository factory.

use std::path::Path;

use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};

/// A target vault's plaintext identity, read without unlocking.
#[derive(Debug, Clone)]
pub struct TargetIdentity {
    pub vault_uuid: Option<String>,
    pub schema_version: i32,
    pub last_unlocked_at: Option<String>,
    /// The live vault's `verify_hash` prefix (16 hex chars) — lets a no-session snapshot
    /// listing flag a snapshot whose credentials differ (a failed rewrap; slice 5.2.1), and
    /// backs the M3 credential warning on a backup preview (slice 5.2.2).
    pub verify_hash_prefix: Option<String>,
    pub entry_count: u64,
    /// The target's rollback `commit_counter` (slice 5.2c). `None` when the column
    /// is absent — a target migrated before 5.2c but not yet reopened. Read as a
    /// SEPARATE best-effort sub-query so its absence can't fail the whole identity
    /// read and silently disable the uuid-mismatch refusal.
    pub commit_counter: Option<i64>,
}

/// What is actually at a target path. See the module docs — collapsing `Missing` and
/// `Unreadable` into one `None` is exactly what finding H2 was.
#[derive(Debug, Clone)]
pub enum TargetState {
    /// Nothing here. For *Replace* this is not an error to confirm — it means the user
    /// wants **Open backup**, which is a different operation.
    Missing,
    /// A vault file exists but its identity could not be read (genuinely corrupt — the
    /// dirty-`-wal` case is handled by the `mode=rw` fallback before we get here).
    /// A caller that overwrites it does so **unidentified**, and must say so out loud.
    Unreadable,
    /// Identified. Every refusal that compares against the target is armed.
    Readable(Box<TargetIdentity>),
}

impl TargetState {
    /// The identity, if it was readable.
    #[must_use]
    pub fn identity(&self) -> Option<&TargetIdentity> {
        match self {
            Self::Readable(id) => Some(id),
            Self::Missing | Self::Unreadable => None,
        }
    }

    /// Is there a vault here at all (readable or not)?
    #[must_use]
    pub const fn exists(&self) -> bool {
        matches!(self, Self::Unreadable | Self::Readable(_))
    }
}

/// Read what is at `home` — three-state (5.2.2). Prefer this over
/// [`read_target_identity`] wherever a refusal depends on the answer.
pub async fn read_target_state(home: &Path) -> TargetState {
    // The arg is the vault home (slice 5.2.0); the `.vdb` lives inside it.
    let vault_file = home.join(crate::domain::shared::VAULT_FILE);
    if !vault_file.exists() {
        return TargetState::Missing;
    }
    let path = vault_file.to_string_lossy();

    // Read-ONLY, always: the caller may be about to replace this vault, so we must not
    // migrate it, WAL-pin it, or write to it in any way. A dirty `-wal` reads fine this
    // way (measured — see the module docs); anything that still fails here is genuinely
    // corrupt, and `Unreadable` says exactly that instead of pretending it isn't there.
    try_read(&format!("sqlite://{path}?mode=ro"))
        .await
        .map_or(TargetState::Unreadable, |id| {
            TargetState::Readable(Box::new(id))
        })
}

/// Read a target's `vault_config` identity + live entry count, read-only.
///
/// Returns `None` when the target is missing **or** unreadable. Retained for the two
/// callers that genuinely only care "can this be opened?" — the launch-screen openability
/// probe and the snapshot listing. 🔴 A caller that *refuses* based on identity must use
/// [`read_target_state`]: `None` here cannot distinguish "no vault" from "unidentified
/// vault", and treating them alike is finding H2.
pub async fn read_target_identity(home: &Path) -> Option<TargetIdentity> {
    match read_target_state(home).await {
        TargetState::Readable(id) => Some(*id),
        TargetState::Missing | TargetState::Unreadable => None,
    }
}

/// Connect with the given URL and read the identity. `None` on any failure — the caller
/// decides what that means.
async fn try_read(url: &str) -> Option<TargetIdentity> {
    let conn = Database::connect(ConnectOptions::new(url.to_owned()))
        .await
        .ok()?;
    let identity = read_inner(&conn).await;
    // Close explicitly so the OS file handle is released before a restore renames the
    // file — Windows refuses to rename a file that still has a live handle, and sqlx's
    // pool close is async (slice 5.2.1: an async close is not a close).
    conn.close().await.ok();
    identity
}

async fn read_inner(conn: &DatabaseConnection) -> Option<TargetIdentity> {
    // `verify_hash` is a Phase-1 column (always present on any migrated vault), so folding
    // it into the identity SELECT is safe — unlike the later `commit_counter` (read below).
    let cfg = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT vault_uuid, schema_version, last_unlocked_at, verify_hash FROM vault_config WHERE id = 'default'",
        ))
        .await
        .ok()??;
    let vault_uuid: Option<String> = cfg.try_get_by_index(0).ok()?;
    let schema_version: i32 = cfg.try_get_by_index(1).ok()?;
    let last_unlocked_at: Option<String> = cfg.try_get_by_index(2).ok()?;
    let verify_hash: Option<Vec<u8>> = cfg.try_get_by_index(3).ok();
    let verify_hash_prefix = verify_hash
        .map(|bytes| crate::infrastructure::snapshot::manifest::verify_hash_prefix(&bytes));
    // Entry count + commit counter are best-effort (preview only); uuid/schema are the
    // ones that gate a refusal, so only they are required. The commit counter is a
    // SEPARATE query (D1/R9): a target migrated before 5.2c lacks the column, and
    // folding it into the SELECT above would fail the whole read → the uuid-mismatch
    // refusal would silently stop firing.
    let entry_count = read_entry_count(conn).await.unwrap_or(0);
    let commit_counter = read_commit_counter(conn).await;
    Some(TargetIdentity {
        vault_uuid,
        schema_version,
        last_unlocked_at,
        verify_hash_prefix,
        entry_count,
        commit_counter,
    })
}

async fn read_commit_counter(conn: &DatabaseConnection) -> Option<i64> {
    let row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT commit_counter FROM vault_config WHERE id = 'default'",
        ))
        .await
        .ok()??;
    row.try_get_by_index(0).ok()
}

async fn read_entry_count(conn: &DatabaseConnection) -> Option<u64> {
    let row = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) FROM entries WHERE is_trashed = 0",
        ))
        .await
        .ok()??;
    let count: i64 = row.try_get_by_index(0).ok()?;
    u64::try_from(count).ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::domain::shared::VAULT_FILE;

    #[tokio::test]
    async fn a_path_with_no_vault_is_missing_not_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            read_target_state(dir.path()).await,
            TargetState::Missing
        ));
    }

    /// 🔴 The `mode=rw` fallback must NEVER be `rwc`: probing a path with no vault must not
    /// CREATE one. (A phantom empty vault at a path the user believes is empty is how a
    /// "restore" silently becomes a "create".)
    #[tokio::test]
    async fn probing_a_missing_vault_does_not_create_one() {
        let dir = tempfile::tempdir().unwrap();
        let _ = read_target_state(dir.path()).await;
        assert!(
            !dir.path().join(VAULT_FILE).exists(),
            "the probe fabricated a vault file"
        );
    }

    #[tokio::test]
    async fn garbage_bytes_read_as_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(VAULT_FILE), b"not a sqlite database at all").unwrap();
        let state = read_target_state(dir.path()).await;
        assert!(matches!(state, TargetState::Unreadable), "got {state:?}");
        assert!(state.exists(), "the file IS there — it just can't be read");
        assert!(state.identity().is_none());
    }
}
