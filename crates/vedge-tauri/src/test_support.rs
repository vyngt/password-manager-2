//! Shared `#[cfg(test)]` helpers for command/state tests.
//!
//! Builds a real [`AppState`] wired to in-memory/real infra (memory keychain +
//! clipboard, on-disk sqlite + blob store under a `TempDir`), plus fast KDF
//! params and a create-a-live-session shortcut. Lives outside `state.rs` so any
//! command test module can reuse it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

use std::sync::Arc;

use vedge_core::application::vault::ports::{
    ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
};
use vedge_core::application::vault::use_cases::{CreateVault, UnlockVault};
use vedge_core::domain::shared::VaultId;

use crate::state::AppState;

/// A real `AppState` backed by in-memory keychain/clipboard and on-disk sqlite +
/// blob store rooted at `dir`.
pub async fn state_fixture(dir: &tempfile::TempDir) -> AppState {
    use vedge_core::application::app::ports::{
        AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository,
        RecentVaultRepository, ThemeRepository,
    };
    use vedge_core::application::vault::ports::BiometricAuthenticator;
    use vedge_core::application::vault::ports::{
        BlobStoreFactory, BreachChecker, VaultRepositoryFactory,
    };
    use vedge_core::infrastructure::biometric::MemoryBiometricAuthenticator;
    use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
    use vedge_core::infrastructure::breach::NoopBreachChecker;
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
    let biometric: Arc<dyn BiometricAuthenticator> = Arc::new(MemoryBiometricAuthenticator::new());
    let clipboard: Arc<dyn ClipboardProvider> = Arc::new(MemoryClipboardProvider::new());
    let breach: Arc<dyn BreachChecker> = Arc::new(NoopBreachChecker::new());

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
    let create_vault = CreateVault {
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
        biometric,
        clipboard,
        breach,
        recent_vaults,
        app_settings,
        themes,
        known_devices,
        extension_sessions,
        unlock_vault,
        create_vault,
    )
}

/// Fast Argon2id params so create tests don't run the 256 MiB production KDF.
/// Mirrors `vedge-core`'s `fast_kdf_params` test helper.
pub fn fast_params() -> vedge_core::domain::vault::kdf_params::KdfParams {
    vedge_core::domain::vault::kdf_params::KdfParams {
        alg: "argon2id".into(),
        m: 8,
        t: 1,
        p: 1,
        version: 1,
    }
}

/// Create a fresh vault under `dir/<filename>`, register its session, and return
/// the state + vault id. The vault ends UNLOCKED with a live, queryable session.
pub async fn unlocked_vault(dir: &tempfile::TempDir, filename: &str) -> (AppState, VaultId) {
    use vedge_core::CreateVaultInput;
    use zeroize::Zeroizing;

    let state = state_fixture(dir).await;
    let vault_path = dir.path().join(filename);
    let vault_id = VaultId::new(vault_path.clone());

    let out = state
        .create_vault
        .execute(CreateVaultInput {
            vault_path,
            master_password: Zeroizing::new("correct horse battery staple".into()),
            secret_key: None,
            kdf_params: Some(fast_params()),
        })
        .await
        .unwrap();
    state.insert_session(vault_id.clone(), out.session).unwrap();
    (state, vault_id)
}
