//! DI root. One function that wires every app-lifetime port and returns a
//! fully-constructed [`AppState`].
//!
//! Call site: the `.setup(|app| …)` closure in [`crate::run`]. The returned
//! state is handed to `app.manage(…)` so commands pull it via
//! `tauri::State<'_, AppState>`.
//!
//! What lives here:
//! - Resolve the platform-specific `app_data_dir` through `tauri::Manager`.
//! - Open the single shared `app.db` connection (migrations run on open).
//! - Wrap each app.db repository in an `Arc<dyn …>` handle.
//! - Construct stateless crypto / KDF / keychain / clipboard providers.
//!
//! What does NOT live here: per-vault state. A `.vdb`'s `VaultRepository`
//! and `BlobStore` are built inside `UnlockVault` and owned by the resulting
//! `VaultSession` — they die when the session drops.

use std::sync::Arc;

use tauri::Manager;

use vedge_core::application::app::ports::{
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository,
    RecentVaultRepository, ThemeRepository,
};
use vedge_core::application::vault::ports::{
    ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
};
use vedge_core::infrastructure::clipboard::ArboardClipboardProvider;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::keychain::OsKeychainProvider;
use vedge_core::infrastructure::sqlite::app::{
    AppDbConnection, SqliteAppSettingRepository, SqliteExtensionSessionRepository,
    SqliteKnownDeviceRepository, SqliteRecentVaultRepository, SqliteThemeRepository,
};

use crate::state::AppState;

/// Error type returned by the tauri `.setup(…)` hook.
pub type ComposeError = Box<dyn std::error::Error + Send + Sync>;

/// Build an [`AppState`] from the running `tauri::App`.
///
/// Runs once at startup. The `app.db` connection, the migrations, and the
/// OS clipboard handle are established here; failing any of them aborts the
/// app rather than shipping a half-working shell.
///
/// The returned future is `!Send` because `tauri::App` holds `!Sync` state
/// (event-loop handles, window proxies). This is intentional — call sites
/// run it via `tauri::async_runtime::block_on` inside the synchronous
/// `.setup(|app| …)` closure, never across threads.
#[allow(clippy::future_not_send)]
pub async fn compose(app: &tauri::App) -> Result<AppState, ComposeError> {
    let app_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_dir)?;

    let app_db_path = app_dir.join("app.db");
    let app_conn = AppDbConnection::open(&app_db_path).await?;

    let recent_vaults: Arc<dyn RecentVaultRepository> =
        Arc::new(SqliteRecentVaultRepository::new(app_conn.handle()));
    let app_settings: Arc<dyn AppSettingRepository> =
        Arc::new(SqliteAppSettingRepository::new(app_conn.handle()));
    let themes: Arc<dyn ThemeRepository> =
        Arc::new(SqliteThemeRepository::new(app_conn.handle()));
    let known_devices: Arc<dyn KnownDeviceRepository> =
        Arc::new(SqliteKnownDeviceRepository::new(app_conn.handle()));
    let extension_sessions: Arc<dyn ExtensionSessionRepository> =
        Arc::new(SqliteExtensionSessionRepository::new(app_conn.handle()));

    let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
    let kdf: Arc<dyn KeyDerivationProvider> = Arc::new(Argon2idKdfProvider::new());
    let keychain: Arc<dyn KeychainProvider> = Arc::new(OsKeychainProvider::new());
    let clipboard: Arc<dyn ClipboardProvider> = Arc::new(ArboardClipboardProvider::new()?);

    Ok(AppState::new(
        crypto,
        kdf,
        keychain,
        clipboard,
        recent_vaults,
        app_settings,
        themes,
        known_devices,
        extension_sessions,
    ))
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
    use vedge_core::domain::shared::VaultId;
    use std::path::PathBuf;

    /// Build an [`AppState`] using the tempdir-backed `app.db` and in-process
    /// providers, bypassing the [`tauri::App`] resolver. Mirrors [`compose`]
    /// for the pieces that don't need a running app handle — exercises the
    /// same wiring so we catch constructor-signature drift even though we
    /// can't instantiate a real [`tauri::App`] in unit tests.
    async fn compose_test_state(dir: &tempfile::TempDir) -> AppState {
        let app_db_path = dir.path().join("app.db");
        let app_conn = AppDbConnection::open(&app_db_path).await.unwrap();

        let recent_vaults: Arc<dyn RecentVaultRepository> =
            Arc::new(SqliteRecentVaultRepository::new(app_conn.handle()));
        let app_settings: Arc<dyn AppSettingRepository> =
            Arc::new(SqliteAppSettingRepository::new(app_conn.handle()));
        let themes: Arc<dyn ThemeRepository> =
            Arc::new(SqliteThemeRepository::new(app_conn.handle()));
        let known_devices: Arc<dyn KnownDeviceRepository> =
            Arc::new(SqliteKnownDeviceRepository::new(app_conn.handle()));
        let extension_sessions: Arc<dyn ExtensionSessionRepository> =
            Arc::new(SqliteExtensionSessionRepository::new(app_conn.handle()));

        let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
        let kdf: Arc<dyn KeyDerivationProvider> = Arc::new(Argon2idKdfProvider::new());
        // OS keychain would hit the real Windows credential vault in tests —
        // swap to the in-memory variant here.
        let keychain: Arc<dyn KeychainProvider> = Arc::new(
            vedge_core::infrastructure::keychain::MemoryKeychainProvider::new(),
        );
        // Clipboard can fail in CI / headless — use the in-memory variant.
        let clipboard: Arc<dyn ClipboardProvider> = Arc::new(
            vedge_core::infrastructure::clipboard::MemoryClipboardProvider::new(),
        );

        AppState::new(
            crypto,
            kdf,
            keychain,
            clipboard,
            recent_vaults,
            app_settings,
            themes,
            known_devices,
            extension_sessions,
        )
    }

    #[tokio::test]
    async fn compose_wires_every_port_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let state = compose_test_state(&dir).await;
        assert!(!state.is_unlocked(&VaultId::new(PathBuf::from("/tmp/does-not-exist.vdb"))));
    }
}
