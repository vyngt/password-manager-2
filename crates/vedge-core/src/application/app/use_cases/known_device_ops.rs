//! Pass-through use cases for `known_devices`. Today these forward to
//! the repo; tomorrow they gain key-rotation hooks, paired-device
//! validation, etc. without touching the shell.

use tracing::instrument;

use crate::application::app::ports::KnownDeviceRepository;
use crate::domain::app::entities::KnownDevice;
use crate::domain::app::errors::AppDbError;
use crate::domain::shared::{DeviceId, now};

#[instrument(skip_all)]
pub async fn list_known_devices(
    repo: &dyn KnownDeviceRepository,
) -> Result<Vec<KnownDevice>, AppDbError> {
    repo.list().await
}

#[instrument(skip_all, fields(id = %device.device_id))]
pub async fn upsert_known_device(
    repo: &dyn KnownDeviceRepository,
    device: &KnownDevice,
) -> Result<(), AppDbError> {
    repo.upsert(device).await
}

#[instrument(skip_all, fields(id = %id))]
pub async fn delete_known_device(
    repo: &dyn KnownDeviceRepository,
    id: &DeviceId,
) -> Result<(), AppDbError> {
    repo.delete(id).await
}

#[instrument(skip_all, fields(id = %id))]
pub async fn touch_known_device_last_seen(
    repo: &dyn KnownDeviceRepository,
    id: &DeviceId,
) -> Result<(), AppDbError> {
    repo.touch_last_seen(id, now()).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::infrastructure::sqlite::app::{AppDbConnection, SqliteKnownDeviceRepository};
    use std::sync::Arc;

    async fn fixture() -> (tempfile::TempDir, Arc<SqliteKnownDeviceRepository>) {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        (dir, Arc::new(SqliteKnownDeviceRepository::new(db.handle())))
    }

    fn sample(id: &str) -> KnownDevice {
        KnownDevice {
            device_id: DeviceId::from_raw(id),
            display_name: "laptop".into(),
            public_key: [7u8; 32],
            first_seen: now(),
            last_seen: None,
        }
    }

    #[tokio::test]
    async fn upsert_then_list_round_trips() {
        let (_dir, repo) = fixture().await;
        upsert_known_device(&*repo, &sample("01ARZ3NDEKTSV4RRFFQ69G5FAV"))
            .await
            .unwrap();
        assert_eq!(list_known_devices(&*repo).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn touch_updates_last_seen() {
        let (_dir, repo) = fixture().await;
        let d = sample("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        upsert_known_device(&*repo, &d).await.unwrap();
        touch_known_device_last_seen(&*repo, &d.device_id)
            .await
            .unwrap();
        let rows = list_known_devices(&*repo).await.unwrap();
        assert!(rows[0].last_seen.is_some());
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let (_dir, repo) = fixture().await;
        let d = sample("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        upsert_known_device(&*repo, &d).await.unwrap();
        delete_known_device(&*repo, &d.device_id).await.unwrap();
        assert_eq!(list_known_devices(&*repo).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_empty_repo_is_empty() {
        let (_dir, repo) = fixture().await;
        assert!(list_known_devices(&*repo).await.unwrap().is_empty());
    }
}
