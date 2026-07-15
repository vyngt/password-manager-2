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
use crate::domain::shared::{StorageError, VAULT_FILE, now};

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
            // `path` is the vault HOME (slice 5.2.0); it "exists" when its `vault.vdb`
            // is a readable file inside the home.
            let exists = v.path.join(VAULT_FILE).is_file();
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
    /// The vault's plaintext `vault_uuid` (slice 5.2.2), so ②'s duplicate check can ask
    /// *"is this identity already registered on this machine?"* without opening every vault.
    ///
    /// Supplied by the **shell**, which probes the vault file with `read_target_state`. The
    /// app-layer use case deliberately does not reach into vault infrastructure to fetch it —
    /// that layering is why this is a parameter and not a lookup. `None` is legitimate (an
    /// unreadable or pre-4.6 vault) and simply means "unknown", never "no uuid".
    pub vault_uuid: Option<String>,
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
    // `input.path` is the vault HOME (slice 5.2.0); the DB lives at `home/vault.vdb`.
    let vault_file = input.path.join(VAULT_FILE);
    if !vault_file.is_file() {
        return Err(AppDbError::InvalidSettingValue {
            key: "recent_vault.path".into(),
            reason: "not a vault home (missing vault.vdb)".into(),
        });
    }

    let mut header = [0u8; 16];
    let mut file = tokio::fs::File::open(&vault_file)
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
        vault_uuid: input.vault_uuid,
    };
    repo.upsert(&vault).await
}

/// Record the `vault_uuid` the vault at this recents row actually has (slice 5.2.2).
///
/// Backfills rows written before the uuid was captured, so ②'s duplicate check gets a complete
/// picture of what this machine holds without waiting for every vault to be re-added. Called on
/// unlock, where the caller has just proved it can read the vault.
///
/// **The vault file is the authority.** If the row already carries a *different* uuid, the row is
/// stale — the vault at that path was converted, replaced from a backup, or swapped — and the
/// value we just read from disk wins. A recents row that disagrees with the vault it points at is
/// worse than one that says nothing: ②'s scan would consult it and reach the wrong conclusion
/// about whether an identity is already present on this machine.
///
/// Best-effort by nature: a missing row is not an error (it may have been removed in another
/// window between the unlock and this write).
#[instrument(skip_all, fields(id = %id))]
pub async fn record_vault_uuid(
    repo: &dyn RecentVaultRepository,
    id: &str,
    vault_uuid: &str,
) -> Result<(), AppDbError> {
    let mut vault = match repo.get(id).await {
        Ok(v) => v,
        // The row vanished (removed in another window between the unlock and this write). That
        // is not a failure of the unlock — there is simply nothing to backfill.
        Err(AppDbError::RecentVaultNotFound(_)) => return Ok(()),
        Err(e) => return Err(e),
    };
    if vault.vault_uuid.as_deref() == Some(vault_uuid) {
        return Ok(());
    }
    vault.vault_uuid = Some(vault_uuid.to_owned());
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

/// Delete every recent-vault row whose vault is no longer on disk. Returns the count of removed
/// rows so the UI can show a "cleaned up N entries" toast.
///
/// 🔴 **This used to test `row.path.is_file()`.** Slice 5.2.0 turned `path` into the vault
/// *home directory*, so `is_file()` became `false` for **every healthy vault** — this function
/// would have deleted the user's entire recents list. It survived only because nothing ever
/// called the command that wraps it. Staleness is now decided the same way as everywhere else:
/// by the presence of `home/vault.vdb`, exactly as `list_recent_vaults_with_status` does.
#[instrument(skip_all)]
pub async fn remove_stale_recents(repo: &dyn RecentVaultRepository) -> Result<u64, AppDbError> {
    let rows = repo.list().await?;
    let mut removed: u64 = 0;
    for row in rows {
        if !row.path.join(VAULT_FILE).is_file() {
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
            vault_uuid: None,
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
        // A vault home with its `vault.vdb` inside (slice 5.2.0).
        let home = dir.path().join("work.vedge");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("vault.vdb"), b"placeholder").unwrap();

        repo.upsert(&RecentVault {
            id: "v1".into(),
            path: home.clone(),
            display_name: "Work".into(),
            last_opened: Some(now()),
            sort_order: 0,
            vault_uuid: None,
        })
        .await
        .unwrap();

        let list = list_recent_vaults_with_status(&*repo).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].exists);
        assert_eq!(list[0].vault.path, home);
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
            vault_uuid: None,
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

    /// Create a vault home with a valid-`SQLite`-header `vault.vdb` inside (slice 5.2.0),
    /// so the header check in `add_recent_vault` passes.
    fn write_vault_home(home: &std::path::Path) {
        std::fs::create_dir_all(home).unwrap();
        write_sqlite_stub(&home.join("vault.vdb"));
    }

    #[tokio::test]
    async fn add_persists_when_header_matches() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("work.vedge");
        write_vault_home(&home);

        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: home.clone(),
                display_name: "Work".into(),
                sort_order: 0,
                vault_uuid: None,
            },
        )
        .await
        .unwrap();

        let rows = repo.list().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].display_name, "Work");
        assert_eq!(rows[0].path, home);
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
                vault_uuid: None,
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
                vault_uuid: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn add_rejects_bad_header() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("fake.vedge");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("vault.vdb"), b"NOT A SQLITE DB!").unwrap(); // 16 wrong bytes
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: home,
                display_name: "x".into(),
                sort_order: 0,
                vault_uuid: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppDbError::InvalidSettingValue { .. }));
    }

    #[tokio::test]
    async fn add_rejects_short_file() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("tiny.vedge");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("vault.vdb"), b"short").unwrap(); // 5 bytes < 16
        let err = add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: home,
                display_name: "x".into(),
                sort_order: 0,
                vault_uuid: None,
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
            vault_uuid: None,
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
        let fresh = dir.path().join("fresh.vedge");
        write_vault_home(&fresh);
        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "fresh".into(),
                path: fresh,
                display_name: "Fresh".into(),
                sort_order: 0,
                vault_uuid: None,
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

    /// 🔴 **Regression guard for a live bug (found in the 5.2.2 scan, not by any test).**
    ///
    /// 5.2.0 turned a recents `path` into the vault **home directory**, but `remove_stale_recents`
    /// kept testing `path.is_file()` — which is `false` for a directory. So the "clean up ghosts"
    /// command would have deleted **every healthy vault in the list**. It never fired only because
    /// no frontend wrapper ever called it, so the bug sat in a registered command, loaded.
    ///
    /// This test seeds a real, healthy vault HOME (as 5.2.0 produces) and demands it survive.
    /// Against the old `is_file()` check it fails immediately.
    #[tokio::test]
    async fn remove_stale_keeps_a_healthy_vault_home_and_removes_ghosts() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("real.vedge");
        write_vault_home(&home); // a DIRECTORY holding vault.vdb — the 5.2.0 layout
        seed(&*repo, "real", home, 0).await;
        seed(&*repo, "ghost", dir.path().join("gone.vedge"), 1).await;

        let n = remove_stale_recents(&*repo).await.unwrap();
        assert_eq!(n, 1, "only the ghost goes");
        let rows = repo.list().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].id, "real",
            "a healthy vault home must NOT be swept away as stale"
        );
    }

    /// A home directory that exists but has lost its `vault.vdb` is a ghost too — staleness is
    /// decided by the vault, not by the folder, and identically to `list_recent_vaults_with_status`.
    #[tokio::test]
    async fn remove_stale_removes_a_home_whose_vault_file_is_gone() {
        let (dir, repo) = fixture().await;
        let hollow = dir.path().join("hollow.vedge");
        std::fs::create_dir_all(&hollow).unwrap(); // the folder is there; the vault is not
        seed(&*repo, "hollow", hollow, 0).await;

        assert_eq!(remove_stale_recents(&*repo).await.unwrap(), 1);
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    // ---- vault_uuid (slice 5.2.2 — ② duplicate detection) --------------------

    #[tokio::test]
    async fn add_persists_the_vault_uuid_and_unlock_backfills_it() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("ident.vedge");
        write_vault_home(&home);

        // A row added WITHOUT a uuid (an older row, or a vault we could not read).
        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: home.clone(),
                display_name: "Work".into(),
                sort_order: 0,
                vault_uuid: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(repo.list().await.unwrap()[0].vault_uuid, None);

        // Unlock reads the vault, so it can fill the gap in.
        record_vault_uuid(&*repo, "v1", "01UUIDUUIDUUIDUUIDUUIDUUID")
            .await
            .unwrap();
        assert_eq!(
            repo.list().await.unwrap()[0].vault_uuid.as_deref(),
            Some("01UUIDUUIDUUIDUUIDUUIDUUID")
        );

        // Backfilling an unknown row is a no-op, not an error: the row may have been removed
        // in another window between the unlock and the write.
        record_vault_uuid(&*repo, "does-not-exist", "01XXXX")
            .await
            .expect("backfilling a vanished row must not fail the unlock that triggered it");

        // And a row added WITH a uuid keeps it.
        let home2 = dir.path().join("known.vedge");
        write_vault_home(&home2);
        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v2".into(),
                path: home2,
                display_name: "Known".into(),
                sort_order: 1,
                vault_uuid: Some("01KNOWNKNOWNKNOWNKNOWNKNOW".into()),
            },
        )
        .await
        .unwrap();
        let v2 = repo.get("v2").await.unwrap();
        assert_eq!(v2.vault_uuid.as_deref(), Some("01KNOWNKNOWNKNOWNKNOWNKNOW"));
    }

    #[tokio::test]
    async fn remove_stale_empties_when_all_ghosts() {
        let (_dir, repo) = fixture().await;
        seed(&*repo, "g1", std::path::PathBuf::from("/no1.vedge"), 0).await;
        seed(&*repo, "g2", std::path::PathBuf::from("/no2.vedge"), 1).await;

        let n = remove_stale_recents(&*repo).await.unwrap();
        assert_eq!(n, 2);
        assert_eq!(repo.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn add_falls_back_to_file_stem_when_name_empty() {
        let (dir, repo) = fixture().await;
        let home = dir.path().join("my-vault.vedge");
        write_vault_home(&home);

        add_recent_vault(
            &*repo,
            AddRecentVaultInput {
                id: "v1".into(),
                path: home,
                display_name: "   ".into(),
                sort_order: 0,
                vault_uuid: None,
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
        // A real vault home with its `vault.vdb` (slice 5.2.0).
        let real = dir.path().join("real.vedge");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("vault.vdb"), b"x").unwrap();

        repo.upsert(&RecentVault {
            id: "ok".into(),
            path: real.clone(),
            display_name: "Real".into(),
            last_opened: None,
            sort_order: 0,
            vault_uuid: None,
        })
        .await
        .unwrap();
        repo.upsert(&RecentVault {
            id: "gone".into(),
            path: std::path::PathBuf::from("/nope.vdb"),
            display_name: "Gone".into(),
            last_opened: None,
            sort_order: 1,
            vault_uuid: None,
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
