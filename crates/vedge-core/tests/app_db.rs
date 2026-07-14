#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use std::path::PathBuf;

use serde_json::json;
use tempfile::tempdir;

use vedge_core::application::app::ports::{
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository, RecentVaultRepository,
    ThemeRepository,
};
use vedge_core::domain::app::entities::{ExtensionSession, KnownDevice, RecentVault, Theme};
use vedge_core::domain::shared::{DeviceId, SessionId, ThemeId, now};
use vedge_core::infrastructure::sqlite::app::{
    AppDbConnection, SqliteAppSettingRepository, SqliteExtensionSessionRepository,
    SqliteKnownDeviceRepository, SqliteRecentVaultRepository, SqliteThemeRepository,
};

#[tokio::test]
async fn migrations_run_and_seed_themes() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("app.db");
    let db = AppDbConnection::open(&db_path).await.unwrap();

    let repo = SqliteThemeRepository::new(db.handle());
    let themes = repo.list().await.unwrap();
    let ids: Vec<_> = themes.iter().map(|t| t.id.as_str().to_owned()).collect();
    assert!(ids.contains(&"builtin-light".to_owned()));
    assert!(ids.contains(&"builtin-dark".to_owned()));
    for t in &themes {
        assert!(t.is_built_in);
        // Semantic colors must be seeded — NULL is reserved for user-created themes
        // that opt into Rust-constant defaults.
        assert_eq!(t.danger_base.as_deref(), Some("#dc2626"));
        assert_eq!(t.warning_base.as_deref(), Some("#d97706"));
        assert_eq!(t.success_base.as_deref(), Some("#16a34a"));
    }
}

#[tokio::test]
async fn recent_vaults_crud_round_trip() {
    let dir = tempdir().unwrap();
    let db = AppDbConnection::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let repo = SqliteRecentVaultRepository::new(db.handle());

    let vault = RecentVault {
        id: "01HV0123456789".to_owned(),
        path: PathBuf::from("/tmp/work.vedge"),
        display_name: "Work".into(),
        last_opened: Some(now()),
        sort_order: 1,
        vault_uuid: Some("01HVUUID0000000000000000".to_owned()),
    };
    repo.upsert(&vault).await.unwrap();

    let fetched = repo.get(&vault.id).await.unwrap();
    assert_eq!(fetched.display_name, "Work");
    assert_eq!(fetched.path, vault.path);
    assert_eq!(fetched.sort_order, 1);

    repo.touch_last_opened(&vault.id, now()).await.unwrap();
    repo.delete(&vault.id).await.unwrap();
    assert!(repo.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn app_settings_store_json_round_trip() {
    let dir = tempdir().unwrap();
    let db = AppDbConnection::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let repo = SqliteAppSettingRepository::new(db.handle());

    repo.set("ui.theme_id", json!("builtin-light"), now())
        .await
        .unwrap();
    repo.set("clipboard.clear_seconds", json!(30), now())
        .await
        .unwrap();

    let theme = repo.get("ui.theme_id").await.unwrap().unwrap();
    assert_eq!(theme.value, json!("builtin-light"));

    let ui = repo.list_prefix("ui.").await.unwrap();
    assert_eq!(ui.len(), 1);
    assert_eq!(ui[0].key, "ui.theme_id");

    repo.delete("ui.theme_id").await.unwrap();
    assert!(repo.get("ui.theme_id").await.unwrap().is_none());
}

#[tokio::test]
async fn known_device_blob_round_trip() {
    let dir = tempdir().unwrap();
    let db = AppDbConnection::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let repo = SqliteKnownDeviceRepository::new(db.handle());

    let device = KnownDevice {
        device_id: DeviceId::from_raw("dev-01"),
        display_name: "Laptop".into(),
        public_key: [7u8; 32],
        first_seen: now(),
        last_seen: None,
    };
    repo.upsert(&device).await.unwrap();

    let fetched = &repo.list().await.unwrap()[0];
    assert_eq!(fetched.public_key, [7u8; 32]);
    assert_eq!(fetched.display_name, "Laptop");

    repo.touch_last_seen(&device.device_id, now())
        .await
        .unwrap();
    let fetched = &repo.list().await.unwrap()[0];
    assert!(fetched.last_seen.is_some());
}

#[tokio::test]
async fn extension_session_round_trip() {
    let dir = tempdir().unwrap();
    let db = AppDbConnection::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let repo = SqliteExtensionSessionRepository::new(db.handle());

    let session = ExtensionSession {
        session_id: SessionId::from_raw("sess-01"),
        browser: "chrome".into(),
        profile_name: Some("default".into()),
        session_key: [3u8; 32],
        created_at: now(),
        last_active_at: None,
    };
    repo.upsert(&session).await.unwrap();

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].session_key, [3u8; 32]);

    repo.delete(&session.session_id).await.unwrap();
    assert!(repo.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn theme_user_can_create_custom_built_in_stays_locked() {
    let dir = tempdir().unwrap();
    let db = AppDbConnection::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let repo = SqliteThemeRepository::new(db.handle());

    let custom = Theme {
        id: ThemeId::from_raw("custom-ocean"),
        name: "Ocean".into(),
        is_built_in: false,
        root_background: "#001F3F".into(),
        root_foreground: "#E6F0FA".into(),
        root_primary: "#7FDBFF".into(),
        danger_base: None,
        warning_base: None,
        success_base: None,
        created_at: now(),
        updated_at: now(),
    };
    repo.upsert(&custom).await.unwrap();
    repo.delete(&custom.id).await.unwrap();

    // Built-in cannot be deleted.
    let builtin = ThemeId::from_raw("builtin-light");
    let err = repo.delete(&builtin).await.unwrap_err();
    assert!(matches!(
        err,
        vedge_core::domain::app::errors::AppDbError::BuiltInThemeImmutable
    ));
}
