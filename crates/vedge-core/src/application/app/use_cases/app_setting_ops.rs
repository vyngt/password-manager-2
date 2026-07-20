//! Pass-through use cases for `app_settings`.
//!
//! Every shell command that touched `AppSettingRepository` now routes
//! through here. Today these are one-liners; tomorrow they're the
//! chokepoint for namespace rules, schema validation, or audit logging.

use serde_json::Value;
use tracing::instrument;

use crate::application::app::ports::AppSettingRepository;
use crate::domain::app::entities::AppSetting;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::now;

#[instrument(skip_all, fields(key = %key))]
pub async fn get_app_setting(
    repo: &dyn AppSettingRepository,
    key: &str,
) -> Result<Option<AppSetting>, AppDbError> {
    repo.get(key).await
}

/// Upsert a setting value. Timestamp is injected from `now()` so the
/// shell doesn't need to supply it.
#[instrument(skip_all, fields(key = %key))]
pub async fn set_app_setting(
    repo: &dyn AppSettingRepository,
    key: &str,
    value: Value,
) -> Result<(), AppDbError> {
    repo.set(key, value, now()).await
}

#[instrument(skip_all, fields(key = %key))]
pub async fn delete_app_setting(
    repo: &dyn AppSettingRepository,
    key: &str,
) -> Result<(), AppDbError> {
    repo.delete(key).await
}

/// List settings whose keys start with `prefix`. Empty prefix lists all.
#[instrument(skip_all, fields(prefix = %prefix))]
pub async fn list_app_settings(
    repo: &dyn AppSettingRepository,
    prefix: &str,
) -> Result<Vec<AppSetting>, AppDbError> {
    repo.list_prefix(prefix).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;
    use crate::infrastructure::sqlite::app::{AppDbConnection, SqliteAppSettingRepository};
    use std::sync::Arc;

    async fn fixture() -> (tempfile::TempDir, Arc<SqliteAppSettingRepository>) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        (dir, Arc::new(SqliteAppSettingRepository::new(db.handle())))
    }

    #[tokio::test]
    async fn set_then_get_round_trips() {
        let (_dir, repo) = fixture().await;
        set_app_setting(&*repo, "ui.panel_width", serde_json::json!(320))
            .await
            .unwrap();
        let got = get_app_setting(&*repo, "ui.panel_width").await.unwrap();
        assert_eq!(got.unwrap().value, serde_json::json!(320));
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let (_dir, repo) = fixture().await;
        set_app_setting(&*repo, "k", serde_json::json!("v"))
            .await
            .unwrap();
        delete_app_setting(&*repo, "k").await.unwrap();
        assert!(get_app_setting(&*repo, "k").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_prefix_filters() {
        let (_dir, repo) = fixture().await;
        set_app_setting(&*repo, "ui.x", serde_json::json!(1))
            .await
            .unwrap();
        set_app_setting(&*repo, "ui.y", serde_json::json!(2))
            .await
            .unwrap();
        set_app_setting(&*repo, "sync.z", serde_json::json!(3))
            .await
            .unwrap();
        let rows = list_app_settings(&*repo, "ui.").await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let (_dir, repo) = fixture().await;
        assert!(get_app_setting(&*repo, "missing").await.unwrap().is_none());
    }
}
