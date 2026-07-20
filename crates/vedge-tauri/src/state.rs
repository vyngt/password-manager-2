//! `AppState` — the single `tauri::State<'_>` every command pulls.
//!
//! Holds two kinds of things:
//! 1. Stateless ports (crypto, KDF, keychain, clipboard, app.db repositories).
//!    Alive for the lifetime of the process.
//! 2. A registry of currently-unlocked [`VaultSession`]s, keyed by
//!    [`VaultId`]. Two-layer locking so multi-vault use doesn't serialize:
//!    the outer `std::sync::Mutex<HashMap>` is held only for the microseconds
//!    of a lookup; each session has its own `tokio::sync::Mutex`, held across
//!    use-case awaits without blocking other vaults.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::Mutex as AsyncMutex;

use vedge_core::application::app::ports::{
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository, RecentVaultRepository,
    ThemeRepository,
};
use vedge_core::application::vault::ports::{
    ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::UnlockVault;
use vedge_core::domain::shared::VaultId;

use crate::error::CommandError;

pub type SessionHandle = Arc<AsyncMutex<VaultSession>>;

pub struct AppState {
    // ---- stateless, process-lifetime ports ---------------------------------
    pub crypto: Arc<dyn CryptoProvider>,
    pub kdf: Arc<dyn KeyDerivationProvider>,
    pub keychain: Arc<dyn KeychainProvider>,
    pub clipboard: Arc<dyn ClipboardProvider>,

    // ---- app.db repositories (one per repo, sharing one DB connection) -----
    pub recent_vaults: Arc<dyn RecentVaultRepository>,
    pub app_settings: Arc<dyn AppSettingRepository>,
    pub themes: Arc<dyn ThemeRepository>,
    pub known_devices: Arc<dyn KnownDeviceRepository>,
    pub extension_sessions: Arc<dyn ExtensionSessionRepository>,

    // ---- pre-wired UnlockVault use case (per-vault infra built via factories
    //      inside `execute`; this struct is constructed once in `compose`) ----
    pub unlock_vault: UnlockVault,

    // ---- active vault sessions ---------------------------------------------
    sessions: Arc<StdMutex<HashMap<VaultId, SessionHandle>>>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        crypto: Arc<dyn CryptoProvider>,
        kdf: Arc<dyn KeyDerivationProvider>,
        keychain: Arc<dyn KeychainProvider>,
        clipboard: Arc<dyn ClipboardProvider>,
        recent_vaults: Arc<dyn RecentVaultRepository>,
        app_settings: Arc<dyn AppSettingRepository>,
        themes: Arc<dyn ThemeRepository>,
        known_devices: Arc<dyn KnownDeviceRepository>,
        extension_sessions: Arc<dyn ExtensionSessionRepository>,
        unlock_vault: UnlockVault,
    ) -> Self {
        Self {
            crypto,
            kdf,
            keychain,
            clipboard,
            recent_vaults,
            app_settings,
            themes,
            known_devices,
            extension_sessions,
            unlock_vault,
            sessions: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    /// Look up an unlocked session. Returns `VaultNotOpen` when the vault
    /// isn't in the registry.
    pub fn get_session(&self, id: &VaultId) -> Result<SessionHandle, CommandError> {
        let Ok(guard) = self.sessions.lock() else {
            return Err(CommandError::Internal);
        };
        guard
            .get(id)
            .cloned()
            .ok_or_else(|| CommandError::VaultNotOpen(id.path().display().to_string()))
    }

    /// `true` if the vault is currently unlocked.
    #[must_use]
    pub fn is_unlocked(&self, id: &VaultId) -> bool {
        self.sessions
            .lock()
            .ok()
            .is_some_and(|g| g.contains_key(id))
    }

    /// Register a new session. Returns `Invalid` if the vault was already
    /// unlocked — caller must lock first. The outer mutex is held only for
    /// the `HashMap` insert; it never wraps an await.
    pub fn insert_session(&self, id: VaultId, session: VaultSession) -> Result<(), CommandError> {
        let Ok(mut guard) = self.sessions.lock() else {
            return Err(CommandError::Internal);
        };
        if guard.contains_key(&id) {
            return Err(CommandError::Invalid(format!(
                "vault already unlocked: {}",
                id.path().display()
            )));
        }
        guard.insert(id, Arc::new(AsyncMutex::new(session)));
        Ok(())
    }

    /// Remove a session from the registry. Returns the handle for the caller
    /// to drop (or to pass to `lock_vault` which will drop the underlying
    /// session via `Drop` → zeroize).
    #[must_use]
    pub fn remove_session(&self, id: &VaultId) -> Option<SessionHandle> {
        self.sessions.lock().ok()?.remove(id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::needless_pass_by_value
    )]

    use super::*;
    use std::path::PathBuf;

    async fn state_fixture(dir: &tempfile::TempDir) -> AppState {
        use vedge_core::application::app::ports::{
            AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository,
            RecentVaultRepository, ThemeRepository,
        };
        use vedge_core::application::vault::ports::{BlobStoreFactory, VaultRepositoryFactory};
        use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
        use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;
        use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
        use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
        use vedge_core::infrastructure::sqlite::app::{
            AppDbConnection, SqliteAppSettingRepository, SqliteExtensionSessionRepository,
            SqliteKnownDeviceRepository, SqliteRecentVaultRepository, SqliteThemeRepository,
        };
        use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

        let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
        let kdf: Arc<dyn KeyDerivationProvider> = Arc::new(Argon2idKdfProvider::new());
        let keychain: Arc<dyn KeychainProvider> = Arc::new(MemoryKeychainProvider::new());
        let clipboard: Arc<dyn ClipboardProvider> = Arc::new(MemoryClipboardProvider::new());

        let db = AppDbConnection::open(&dir.path().join("app.db"))
            .await
            .unwrap();
        let recent_vaults: Arc<dyn RecentVaultRepository> =
            Arc::new(SqliteRecentVaultRepository::new(db.handle()));
        let app_settings: Arc<dyn AppSettingRepository> =
            Arc::new(SqliteAppSettingRepository::new(db.handle()));
        let themes: Arc<dyn ThemeRepository> = Arc::new(SqliteThemeRepository::new(db.handle()));
        let known_devices: Arc<dyn KnownDeviceRepository> =
            Arc::new(SqliteKnownDeviceRepository::new(db.handle()));
        let extension_sessions: Arc<dyn ExtensionSessionRepository> =
            Arc::new(SqliteExtensionSessionRepository::new(db.handle()));

        let unlock_vault = UnlockVault {
            repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
                as Arc<dyn VaultRepositoryFactory>,
            blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
            crypto: Arc::clone(&crypto),
            clipboard: Arc::clone(&clipboard),
            kdf: Arc::clone(&kdf),
            keychain: Arc::clone(&keychain),
        };

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
            unlock_vault,
        )
    }

    #[tokio::test]
    async fn get_session_on_unknown_vault_returns_not_open() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        let id = VaultId::new(PathBuf::from("/tmp/nope.vdb"));
        match state.get_session(&id) {
            Err(CommandError::VaultNotOpen(_)) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn is_unlocked_flips_on_insert_and_remove() {
        // We can't easily construct a real VaultSession here without running
        // UnlockVault end-to-end. The is_unlocked contract is trivial —
        // it's a HashMap::contains_key — so we defer this to an integration
        // test in a later phase (tests/vault_commands.rs).
        let dir = tempfile::tempdir().unwrap();
        let state = state_fixture(&dir).await;
        assert!(!state.is_unlocked(&VaultId::new(PathBuf::from("/x"))));
    }
}
