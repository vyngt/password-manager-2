use std::path::Path;
use std::sync::Arc;

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::domain::shared::StorageError;

use super::migrations::Migrator;

#[derive(Clone)]
pub struct VaultDbConnection {
    conn: Arc<DatabaseConnection>,
}

impl VaultDbConnection {
    pub async fn open(path: &Path) -> Result<Self, StorageError> {
        let url = format!("sqlite://{}?mode=rwc", path.to_string_lossy());
        let conn = Database::connect(ConnectOptions::new(url))
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        // Pin WAL (slice 5.2). `journal_mode=WAL` is persistent — it lives in the
        // `.vdb` header, so this one-shot execute survives reopen and flips any
        // pre-5.2 vault on its next open. Backup correctness (a live `VACUUM INTO`
        // snapshot) assumes WAL; it was never pinned before. Set BEFORE migrations
        // so the very first write runs in WAL. (`busy_timeout`/`foreign_keys` are
        // per-connection and sea-orm 1.1 exposes no `after_connect` hook — out of
        // scope here; WAL is the only correctness-critical pragma for backup.)
        conn.execute_unprepared("PRAGMA journal_mode=WAL;")
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Migrator::up(&conn, None)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(conn),
        })
    }

    #[must_use]
    pub fn handle(&self) -> Arc<DatabaseConnection> {
        Arc::clone(&self.conn)
    }

    /// Cleanly close the pool, releasing the OS file handle (Windows will not let another
    /// process rename/delete the `.vdb` while a handle is open) and checkpointing the WAL
    /// into the main file. Consumes `self`; a no-op if other [`handle`](Self::handle)
    /// clones still outlive this one.
    pub async fn close(self) -> Result<(), StorageError> {
        match Arc::try_unwrap(self.conn) {
            Ok(conn) => conn
                .close()
                .await
                .map_err(|e| StorageError::Database(e.to_string())),
            Err(_) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    use super::VaultDbConnection;

    /// A freshly-opened vault is in WAL mode (slice 5.2 pins it in `open`).
    /// `journal_mode` is persistent, so this also proves the header carries it.
    #[tokio::test]
    async fn open_pins_wal_journal_mode() {
        let dir = tempfile::tempdir().unwrap();
        let db = VaultDbConnection::open(&dir.path().join("wal.vdb"))
            .await
            .unwrap();
        let row = db
            .handle()
            .query_one(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA journal_mode;",
            ))
            .await
            .unwrap()
            .expect("PRAGMA journal_mode returns a row");
        let mode: String = row.try_get_by_index(0).unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
}
