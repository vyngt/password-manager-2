//! Read a target vault's plaintext identity WITHOUT unlocking it (slice 5.2b).
//!
//! Restore and inspect must compare a backup's `vault_uuid` / `schema_version`
//! against the vault they are about to overwrite. Those are ordinary (unencrypted)
//! `vault_config` columns, so a read-only `SELECT` — no KEK, no migration, no write
//! — is enough. Opened `mode=ro` precisely because the caller may be about to
//! replace this file: it must not migrate or `WAL`-pin the very file it will
//! discard. A dirty-`WAL` (crashed) target may read as `None` ("unreadable"); the
//! restore precondition is a *cleanly locked* vault, which reads fine.

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
    pub entry_count: u64,
}

/// Read a target `.vdb`'s `vault_config` identity + live entry count, read-only.
///
/// Returns `None` when the target is missing or unreadable (the corrupt-vault
/// case) — callers treat that as "identity unknown", never a hard error.
pub async fn read_target_identity(vault_path: &Path) -> Option<TargetIdentity> {
    if !vault_path.exists() {
        return None;
    }
    let url = format!("sqlite://{}?mode=ro", vault_path.to_string_lossy());
    let conn = Database::connect(ConnectOptions::new(url)).await.ok()?;
    let identity = read_inner(&conn).await;
    // Close explicitly so the OS file handle is released before a restore renames
    // the file (Windows refuses to rename a file that still has a live handle).
    conn.close().await.ok();
    identity
}

async fn read_inner(conn: &DatabaseConnection) -> Option<TargetIdentity> {
    let cfg = conn
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT vault_uuid, schema_version, last_unlocked_at FROM vault_config WHERE id = 'default'",
        ))
        .await
        .ok()??;
    let vault_uuid: Option<String> = cfg.try_get_by_index(0).ok()?;
    let schema_version: i32 = cfg.try_get_by_index(1).ok()?;
    let last_unlocked_at: Option<String> = cfg.try_get_by_index(2).ok()?;
    // Entry count is best-effort (preview only); uuid/schema are the ones that gate
    // a refusal, so only they are required.
    let entry_count = read_entry_count(conn).await.unwrap_or(0);
    Some(TargetIdentity {
        vault_uuid,
        schema_version,
        last_unlocked_at,
        entry_count,
    })
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
