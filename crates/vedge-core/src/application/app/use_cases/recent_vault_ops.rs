//! `list_recent_vaults_with_status` — list recents + flag stale paths.
//!
//! The onboarding screen shows the user their recent vaults and needs to
//! render a visual cue ("File not found — remove?") for entries whose
//! path no longer exists on disk. We compute `exists` in the use case so
//! the shell doesn't have to replicate the check, and so tests can
//! exercise the logic against tempdir paths.

use std::path::PathBuf;

use tokio::io::AsyncReadExt;
use tracing::instrument;

use crate::application::app::ports::RecentVaultRepository;
use crate::domain::app::entities::RecentVault;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{StorageError, now};

/// First 16 bytes of every `SQLite` 3 database. Used as a cheap
/// pre-flight check when adding a vault to the recents list — full
/// VEDG verification happens later at unlock time.
///
/// See <https://www.sqlite.org/fileformat.html#magic_header_string>.
pub const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// A recent vault row plus a filesystem existence flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentVaultStatus {
    pub vault: RecentVault,
    /// `true` if `vault.path` resolves to a readable file on disk at
    /// list time.
    pub exists: bool,
}

/// List every recent vault, each tagged with whether its path exists on
/// disk right now.
///
/// The repository returns rows newest-first by `last_opened DESC` (rows
/// never opened — `NULL` — sort last); the shell re-sorts client-side to
/// also push missing files to the bottom.
#[instrument(skip_all)]
pub async fn list_recent_vaults_with_status(
    repo: &dyn RecentVaultRepository,
) -> Result<Vec<RecentVaultStatus>, AppDbError> {
    let rows = repo.list().await?;
    Ok(rows
        .into_iter()
        .map(|v| {
            let exists = v.path.is_file();
            RecentVaultStatus { vault: v, exists }
        })
        .collect())
}

// ---- Pass-through ops --------------------------------------------------------

/// List every recent vault. Pass-through to the repo.
#[instrument(skip_all)]
pub async fn list_recent_vaults(
    repo: &dyn RecentVaultRepository,
) -> Result<Vec<RecentVault>, AppDbError> {
    repo.list().await
}

/// Remove a recent-vault row. Pass-through to the repo.
#[instrument(skip_all, fields(id = %id))]
pub async fn remove_recent_vault(
    repo: &dyn RecentVaultRepository,
    id: &str,
) -> Result<(), AppDbError> {
    repo.delete(id).await
}

/// Bump a recent-vault row's `last_opened` to now. Recency (`last_opened
/// DESC`) is the single ordering key, so this alone floats the row to the
/// top on next list. Equivalent to [`touch_on_unlock`].
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_recent_vault(
    repo: &dyn RecentVaultRepository,
    id: &str,
) -> Result<(), AppDbError> {
    repo.touch_last_opened(id, now()).await
}

/// Input to [`add_recent_vault`].
#[derive(Debug, Clone)]
pub struct AddRecentVaultInput {
    /// Caller-supplied ID. Typically a fresh ULID from the frontend.
    pub id: String,
    pub path: PathBuf,
    /// Human-readable name. Trimmed; if empty, falls back to the path's
    /// file stem.
    pub display_name: String,
    pub sort_order: i32,
}

/// Register a vault in the recents list.
///
/// Validates that `path` is a regular file with a `SQLite` header
/// before persisting — catches obvious typos + non-database files at
/// the boundary, without opening the DB. Full VEDG verification happens
/// later when the user tries to unlock.
///
/// Stamps `last_opened = now()`: adding a vault means it was just created
/// or just opened, so it should sort to the **top** of the recency list.
#[instrument(skip_all, fields(path = %input.path.display()))]
pub async fn add_recent_vault(
    repo: &dyn RecentVaultRepository,
    input: AddRecentVaultInput,
) -> Result<(), AppDbError> {
    if !input.path.is_file() {
        return Err(AppDbError::InvalidSettingValue {
            key: "recent_vault.path".into(),
            reason: "not a regular file".into(),
        });
    }

    let mut header = [0u8; 16];
    let mut file = tokio::fs::File::open(&input.path)
        .await
        .map_err(|e| AppDbError::Storage(StorageError::Io(e.to_string())))?;
    match file.read_exact(&mut header).await {
        Ok(n) if n == header.len() => {}
        Ok(_) => {
            // `read_exact` returns Ok(n) == buf.len() on success, but
            // treat anything else as a short read defensively.
            return Err(AppDbError::InvalidSettingValue {
                key: "recent_vault.path".into(),
                reason: "file shorter than SQLite header".into(),
            });
        }
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(AppDbError::InvalidSettingValue {
                key: "recent_vault.path".into(),
                reason: "file shorter than SQLite header".into(),
            });
        }
        Err(e) => {
            return Err(AppDbError::Storage(StorageError::Io(e.to_string())));
        }
    }
    if &header != SQLITE_MAGIC {
        return Err(AppDbError::InvalidSettingValue {
            key: "recent_vault.path".into(),
            reason: "not a sqlite database (bad header)".into(),
        });
    }

    let trimmed = input.display_name.trim();
    let display_name = if trimmed.is_empty() {
        input
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("vault")
            .to_owned()
    } else {
        trimmed.to_owned()
    };

    let vault = RecentVault {
        id: input.id,
        path: input.path,
        display_name,
        last_opened: Some(now()),
        sort_order: input.sort_order,
    };
    repo.upsert(&vault).await
}

/// Rename a recent vault's display name.
///
/// Trims the input and rejects an all-whitespace name (unlike
/// [`add_recent_vault`], which falls back to the file stem — a rename is
/// an explicit edit, so an empty name is a user error, not a default). A
/// file-opened vault is named from its file stem; this is the one
/// "vault setting" that surface needs.
#[instrument(skip_all, fields(id = %id))]
pub async fn rename_recent_vault(
    repo: &dyn RecentVaultRepository,
    id: &str,
    display_name: &str,
) -> Result<(), AppDbError> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Err(AppDbError::InvalidSettingValue {
            key: "recent_vault.display_name".into(),
            reason: "display name must not be empty".into(),
        });
    }
    let mut vault = repo.get(id).await?;
    vault.display_name = trimmed.to_owned();
    repo.upsert(&vault).await
}

/// Called by the shell immediately after a successful unlock.
///
/// Bumps the target row's `last_opened` to now. Recency (`last_opened
/// DESC`) is the single ordering key, so the freshly-opened vault rises
/// to the top of the next list with no `sort_order` bookkeeping. Errors
/// with [`AppDbError::RecentVaultNotFound`] if the id is unknown.
#[instrument(skip_all, fields(id = %id))]
pub async fn touch_on_unlock(repo: &dyn RecentVaultRepository, id: &str) -> Result<(), AppDbError> {
    repo.touch_last_opened(id, now()).await
}

/// Delete every recent-vault row whose path no longer resolves to a
/// regular file. Returns the count of removed rows so the UI can show a
/// "cleaned up N entries" toast.
#[instrument(skip_all)]
pub async fn remove_stale_recents(repo: &dyn RecentVaultRepository) -> Result<u64, AppDbError> {
    let rows = repo.list().await?;
    let mut removed: u64 = 0;
    for row in rows {
        if !row.path.is_file() {
            repo.delete(&row.id).await?;
            removed = removed.saturating_add(1);
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use super::*;
    use crate::domain::shared::{Timestamp, now};
    use crate::infrastructure::sqlite::app::{AppDbConnection, SqliteRecentVaultRepository};
    use std::sync::Arc;

    async fn fixture() -> (tempfile::TempDir, Arc<SqliteRecentVaultRepository>) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        let repo = Arc::new(SqliteRecentVaultRepository::new(db.handle()));
        (dir, repo)
    }

    /// A fixed RFC3339-UTC instant for deterministic recency ordering.
    fn ts(rfc3339: &str) -> Timestamp {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    /// Seed a row with an explicit `last_opened` (control recency ordering).
    async fn seed_at(
        repo: &dyn RecentVaultRepository,
        id: &str,
        path: std::path::PathBuf,
        last_opened: Option<Timestamp>,
    ) {
        repo.upsert(&RecentVault {
            id: id.into(),
            path,
            display_name: id.into(),
            last_opened,
            sort_order: 0,
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn empty_repo_returns_empty_list() {
        let (_dir, repo) = fixture().await;
        let list = list_recent_vaults_with_status(&*repo).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn existing_path_flagged_true() {
        let (dir, repo) = fixture().await;
        let vdb = dir.path().join("work.vdb");
        std::fs::write(&vdb, b"placeholder").unwrap();

        repo.upsert(&RecentVault {
            id: "v1".into(),
            path: vdb.clone(),
            display_name: "Work".into(),
            last_opened: Some(now()),
            sort_order: 0,
        })
        .await
        .unwrap();

        let list = list_recent_vaults_with_status(&*repo).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].exists);
        assert_eq!(list[0].vault.path, vdb);
    }

    #[tokio::test]
    async fn missing_path_flagged_false() {
        let (_dir, repo) = fixture().await;
        repo.upsert(&RecentVault {
            id: "gone".into(),
            path: std::path::PathBuf::from("/definitely/does/not/exist.vdb"),
            display_name: "Ghost".into(),
            last_opened: None,
            sort_order: 0,
        })
        .await
        .unwrap();

        let list = list_recent_vaults_with_status(&*repo).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].exists);
    }

    // ---- add_recent_vault ---------------------------------------------------

    /// Write a file that starts with the `SQLite` header so the magic
    /// check in `add_recent_vault` passes. We don't need a valid
    /// database — the use case only reads the first 16 bytes.
    fn write_sqlite_stub(path: &std::path::Path) {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(SQLITE_MAGIC);
        bytes.extend_from_slice(&[0u8; 16]);
        std::fs::write(path, bytes).unwrap();
    }

    #[tokio::test]
    async fn add_persists_when_header_matches() {
        let (dir, repo) = fixture().await;
        let vdb = dir.path().join("work.vdb");
        write_sqlite_stub(&vdb);

        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: vdb.clone(),
                display_name: "Work".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].display_name, "Work");
        assert_eq!(rows[0].path, vdb);
        // Adding stamps `last_opened = now()` so the vault sorts to the top.
        assert!(rows[0].last_opened.is_some());
    }

    // ---- rename_recent_vault ------------------------------------------------

    #[tokio::test]
    async fn rename_recent_vault_persists() {
        let (dir, repo) = fixture().await;
        seed_at(&*repo, "v1", dir.path().join("v.vdb"), None).await;

        rename_recent_vault(&*repo, "v1", "  My Vault  ")
            .await
            .unwrap();

        // Trimmed and persisted.
        assert_eq!(repo.get("v1").await.unwrap().display_name, "My Vault");
    }

    #[tokio::test]
    async fn rename_rejects_empty() {
        let (dir, repo) = fixture().await;
        seed_at(&*repo, "v1", dir.path().join("v.vdb"), None).await;

        let err = rename_recent_vault(&*repo, "v1", "   ").await.unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
        // Original name left untouched (seed set it to the id).
        assert_eq!(repo.get("v1").await.unwrap().display_name, "v1");
    }

    #[tokio::test]
    async fn rename_errors_on_missing_id() {
        let (_dir, repo) = fixture().await;
        let err = rename_recent_vault(&*repo, "ghost", "X").await.unwrap_err();
        assert!(matches!(err, AppDbError::RecentVaultNotFound(_)));
    }

    #[tokio::test]
    async fn add_rejects_missing_path() {
        let (_dir, repo) = fixture().await;
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: std::path::PathBuf::from("/definitely/does/not/exist.vdb"),
                display_name: "x".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn add_rejects_directory() {
        let (dir, repo) = fixture().await;
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: dir.path().to_path_buf(),
                display_name: "x".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn add_rejects_bad_header() {
        let (dir, repo) = fixture().await;
        let p = dir.path().join("fake.vdb");
        std::fs::write(&p, b"NOT A SQLITE DB!").unwrap(); // exactly 16 wrong bytes
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: p,
                display_name: "x".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn add_rejects_short_file() {
        let (dir, repo) = fixture().await;
        let p = dir.path().join("tiny.vdb");
        std::fs::write(&p, b"short").unwrap(); // 5 bytes < 16
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: p,
                display_name: "x".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    // ---- touch_on_unlock ---------------------------------------------------

    async fn seed(
        repo: &dyn RecentVaultRepository,
        id: &str,
        path: std::path::PathBuf,
        sort_order: i32,
    ) {
        repo.upsert(&RecentVault {
            id: id.into(),
            path,
            display_name: id.into(),
            last_opened: None,
            sort_order,
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn touch_on_unlock_moves_to_top_and_stamps_last_opened() {
        let (dir, repo) = fixture().await;
        seed_at(
            &*repo,
            "a",
            dir.path().join("a.vdb"),
            Some(ts("2020-01-01T00:00:00+00:00")),
        )
        .await;
        seed_at(
            &*repo,
            "b",
            dir.path().join("b.vdb"),
            Some(ts("2021-01-01T00:00:00+00:00")),
        )
        .await;
        seed_at(
            &*repo,
            "c",
            dir.path().join("c.vdb"),
            Some(ts("2022-01-01T00:00:00+00:00")),
        )
        .await;

        // Before: newest-first is c, b, a.
        let before: Vec<_> = repo
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(before, ["c", "b", "a"]);

        // Opening 'a' stamps now() (later than any seeded year) → floats to top.
        touch_on_unlock(&*repo, "a").await.unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows[0].id, "a");
        assert!(rows[0].last_opened.is_some());
    }

    #[tokio::test]
    async fn touch_on_unlock_errors_on_missing_id() {
        let (_dir, repo) = fixture().await;
        let err = touch_on_unlock(&*repo, "ghost").await.unwrap_err();
        assert!(matches!(err, AppDbError::RecentVaultNotFound(_)));
    }

    #[tokio::test]
    async fn touch_on_unlock_stamps_single_row() {
        let (dir, repo) = fixture().await;
        seed_at(&*repo, "a", dir.path().join("a.vdb"), None).await;

        touch_on_unlock(&*repo, "a").await.unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].last_opened.is_some());
    }

    #[tokio::test]
    async fn list_orders_by_recency_nulls_last() {
        let (dir, repo) = fixture().await;
        seed_at(
            &*repo,
            "mid",
            dir.path().join("mid.vdb"),
            Some(ts("2021-01-01T00:00:00+00:00")),
        )
        .await;
        seed_at(
            &*repo,
            "new",
            dir.path().join("new.vdb"),
            Some(ts("2022-01-01T00:00:00+00:00")),
        )
        .await;
        seed_at(&*repo, "never", dir.path().join("never.vdb"), None).await;

        let order: Vec<_> = repo
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(order, ["new", "mid", "never"]);
    }

    #[tokio::test]
    async fn created_vault_is_most_recent() {
        let (dir, repo) = fixture().await;
        // A previously-opened vault, long ago.
        seed_at(
            &*repo,
            "old",
            dir.path().join("old.vdb"),
            Some(ts("2020-01-01T00:00:00+00:00")),
        )
        .await;

        // Adding a brand-new vault stamps last_opened = now().
        let fresh = dir.path().join("fresh.vdb");
        write_sqlite_stub(&fresh);
        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "fresh".into(),
                path: fresh,
                display_name: "Fresh".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows[0].id, "fresh");
    }

    // ---- remove_stale_recents ----------------------------------------------

    #[tokio::test]
    async fn remove_stale_on_empty_returns_zero() {
        let (_dir, repo) = fixture().await;
        assert_eq!(remove_stale_recents(&*repo).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn remove_stale_keeps_real_removes_ghost() {
        let (dir, repo) = fixture().await;
        let real = dir.path().join("real.vdb");
        std::fs::write(&real, b"x").unwrap();
        seed(&*repo, "real", real, 0).await;
        seed(&*repo, "ghost", std::path::PathBuf::from("/nope.vdb"), 1).await;

        let n = remove_stale_recents(&*repo).await.unwrap();
        assert_eq!(n, 1);
        let rows = repo.list().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "real");
    }

    #[tokio::test]
    async fn remove_stale_empties_when_all_ghosts() {
        let (_dir, repo) = fixture().await;
        seed(&*repo, "g1", std::path::PathBuf::from("/no1.vdb"), 0).await;
        seed(&*repo, "g2", std::path::PathBuf::from("/no2.vdb"), 1).await;

        let n = remove_stale_recents(&*repo).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn add_falls_back_to_file_stem_when_name_empty() {
        let (dir, repo) = fixture().await;
        let p = dir.path().join("my-vault.vdb");
        write_sqlite_stub(&p);

        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: p,
                display_name: "   ".into(),
                sort_order: 0,
            },
        )
        .await
        .unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows[0].display_name, "my-vault");
    }

    #[tokio::test]
    async fn mixed_statuses_report_independently() {
        let (dir, repo) = fixture().await;
        let real = dir.path().join("real.vdb");
        std::fs::write(&real, b"x").unwrap();

        repo.upsert(&RecentVault {
            id: "ok".into(),
            path: real.clone(),
            display_name: "Real".into(),
            last_opened: None,
            sort_order: 0,
        })
        .await
        .unwrap();
        repo.upsert(&RecentVault {
            id: "gone".into(),
            path: std::path::PathBuf::from("/nope.vdb"),
            display_name: "Gone".into(),
            last_opened: None,
            sort_order: 1,
        })
        .await
        .unwrap();

        let list = list_recent_vaults_with_status(&*repo).await.unwrap();
        assert_eq!(list.len(), 2);
        let ok = list.iter().find(|s| s.vault.id == "ok").unwrap();
        let gone = list.iter().find(|s| s.vault.id == "gone").unwrap();
        assert!(ok.exists);
        assert!(!gone.exists);
    }
}
