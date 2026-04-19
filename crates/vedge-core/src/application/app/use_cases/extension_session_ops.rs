//! Pass-through use cases for `extension_sessions`.
//!
//! Today these forward to the repo; future work (session-key rotation,
//! TTL-based cleanup, paired-device enforcement) lands here without
//! touching the shell.

use tracing::instrument;

use crate::application::app::ports::ExtensionSessionRepository;
use crate::domain::app::entities::ExtensionSession;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{now, SessionId};

#[instrument(skip_all)]
pub async fn list_extension_sessions(
    repo: &dyn ExtensionSessionRepository,
) -> Result<Vec<ExtensionSession>, AppDbError> {
    repo.list().await
}

#[instrument(skip_all, fields(id = %session.session_id))]
pub async fn upsert_extension_session(
    repo: &dyn ExtensionSessionRepository,
    session: &ExtensionSession,
) -> Result<(), AppDbError> {
    repo.upsert(session).await
}

#[instrument(skip_all, fields(id = %id))]
pub async fn delete_extension_session(
    repo: &dyn ExtensionSessionRepository,
    id: &SessionId,
) -> Result<(), AppDbError> {
    repo.delete(id).await
}

#[instrument(skip_all, fields(id = %id))]
pub async fn touch_extension_session_last_active(
    repo: &dyn ExtensionSessionRepository,
    id: &SessionId,
) -> Result<(), AppDbError> {
    repo.touch_last_active(id, now()).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::infrastructure::sqlite::app::{AppDbConnection, SqliteExtensionSessionRepository};
    use std::sync::Arc;

    async fn fixture() -> (tempfile::TempDir, Arc<SqliteExtensionSessionRepository>) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        (
            dir,
            Arc::new(SqliteExtensionSessionRepository::new(db.handle())),
        )
    }

    fn sample(id: &str) -> ExtensionSession {
        ExtensionSession {
            session_id: SessionId::from_raw(id),
            browser: "firefox".into(),
            profile_name: Some("default".into()),
            session_key: [9u8; 32],
            created_at: now(),
            last_active_at: None,
        }
    }

    #[tokio::test]
    async fn upsert_then_list_round_trips() {
        let (_dir, repo) = fixture().await;
        upsert_extension_session(&*repo, &sample("01ARZ3NDEKTSV4RRFFQ69G5FAV"))
            .await
            .unwrap();
        assert_eq!(list_extension_sessions(&*repo).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn touch_updates_last_active() {
        let (_dir, repo) = fixture().await;
        let s = sample("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        upsert_extension_session(&*repo, &s).await.unwrap();
        touch_extension_session_last_active(&*repo, &s.session_id)
            .await
            .unwrap();
        let rows = list_extension_sessions(&*repo).await.unwrap();
        assert!(rows[0].last_active_at.is_some());
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let (_dir, repo) = fixture().await;
        let s = sample("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        upsert_extension_session(&*repo, &s).await.unwrap();
        delete_extension_session(&*repo, &s.session_id)
            .await
            .unwrap();
        assert_eq!(list_extension_sessions(&*repo).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_empty_repo_is_empty() {
        let (_dir, repo) = fixture().await;
        assert!(list_extension_sessions(&*repo).await.unwrap().is_empty());
    }
}
