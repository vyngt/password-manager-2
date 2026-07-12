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
    AppSettingRepository, ExtensionSessionRepository, KnownDeviceRepository, RecentVaultRepository,
    ThemeRepository,
};
use vedge_core::application::vault::ports::{
    BiometricAuthenticator, BlobStoreFactory, BreachChecker, ClipboardProvider, CryptoProvider,
    KeyDerivationProvider, KeychainProvider, VaultRepositoryFactory,
};
use vedge_core::application::vault::use_cases::{CreateVault, UnlockVault};
use vedge_core::infrastructure::biometric::platform_authenticator;
use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
use vedge_core::infrastructure::clipboard::ArboardClipboardProvider;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::keychain::OsKeychainProvider;
use vedge_core::infrastructure::sqlite::app::{
    AppDbConnection, SqliteAppSettingRepository, SqliteExtensionSessionRepository,
    SqliteKnownDeviceRepository, SqliteRecentVaultRepository, SqliteThemeRepository,
};
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

use crate::breach::HibpBreachChecker;
use crate::state::AppState;

/// Error type returned by the tauri `.setup(…)` hook.
pub type ComposeError = Box<dyn std::error::Error + Send + Sync>;

/// Resolve the directory that holds `app.db` and user-writable files.
///
/// - **`VEDGE_DATA_DIR` env override** (any build): if set and non-empty, that
///   path wins — checked *first*, before the debug/release branch. Enables a
///   portable/relocatable data directory and, in particular, lets the e2e
///   harness point `app.db` at a per-run temp dir for hermetic, repeatable
///   runs (see `crates/vedge-e2e`). Paired with disabling the single-instance
///   lock (`crate::run`) so a portable/test instance doesn't collide with a
///   normally-installed one.
/// - **Debug builds** (`cargo run`, `cargo tauri dev`): `<workspace>/local/`.
///   Keeps dev data out of `%APPDATA%` / `~/.config` so wiping state is a
///   simple `rm -rf local/`, and multiple checkouts don't clobber each
///   other. The folder is gitignored.
/// - **Release builds** (`cargo build --release`, `cargo tauri build`):
///   Tauri's platform-resolved `app_data_dir()` — `%APPDATA%\com.vedge.app`
///   on Windows, `~/Library/Application Support/...` on macOS,
///   `~/.local/share/...` on Linux.
///
/// Absent the env override the switch is compile-time (`cfg!(debug_assertions)`)
/// so release binaries never accidentally read/write the dev folder.
fn resolve_app_dir(app: &tauri::App) -> Result<std::path::PathBuf, ComposeError> {
    if let Some(dir) = std::env::var_os("VEDGE_DATA_DIR").filter(|v| !v.is_empty()) {
        return Ok(std::path::PathBuf::from(dir));
    }
    if cfg!(debug_assertions) {
        // `env!("CARGO_MANIFEST_DIR")` resolves at compile time to this
        // crate's absolute path. `ancestors().nth(2)` walks up
        // `crates/vedge-tauri` → `crates/` → `<workspace>`.
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace = manifest.ancestors().nth(2).unwrap_or(manifest);
        Ok(workspace.join("local"))
    } else {
        Ok(app.path().app_data_dir()?)
    }
}

/// Select the biometric authenticator. **Production always uses the platform
/// authenticator** (`platform_authenticator` → Windows Hello / a no-op stub). The
/// e2e/test path may swap in the deterministic in-memory
/// `MemoryBiometricAuthenticator` (reports available, never prompts) so the
/// enroll → lock → unlock-via-biometric flow runs headlessly — but only under
/// **two** gates that can't both hold in a shipped bundle: a debug build
/// (`cfg(debug_assertions)`, absent in the release bundle) **and** the explicit
/// `VEDGE_E2E_BIOMETRIC_MEMORY` env var (set by the e2e harness). Mirrors
/// `resolve_app_dir`'s env-first, cfg-gated seam.
#[cfg(debug_assertions)]
fn resolve_biometric(crypto: Arc<dyn CryptoProvider>) -> Arc<dyn BiometricAuthenticator> {
    if std::env::var_os("VEDGE_E2E_BIOMETRIC_MEMORY").is_some_and(|v| !v.is_empty()) {
        return Arc::new(
            vedge_core::infrastructure::biometric::MemoryBiometricAuthenticator::new(),
        );
    }
    platform_authenticator(crypto)
}

#[cfg(not(debug_assertions))]
fn resolve_biometric(crypto: Arc<dyn CryptoProvider>) -> Arc<dyn BiometricAuthenticator> {
    platform_authenticator(crypto)
}

/// Select the breach checker (slice 4.4). **Production always uses the real
/// `HibpBreachChecker`** (an HTTPS k-anonymity call). Under the same two-gate seam
/// as [`resolve_biometric`] — a debug build (`cfg(debug_assertions)`) **and** the
/// explicit `VEDGE_E2E_BREACH_MEMORY` env var — the e2e harness swaps in an offline
/// `MemoryBreachChecker` preloaded with one known-breached password, so
/// `scan_health_shows_breached` runs headlessly with no network. Neither gate can
/// hold in a shipped bundle.
#[cfg(debug_assertions)]
fn resolve_breach() -> Arc<dyn BreachChecker> {
    if std::env::var_os("VEDGE_E2E_BREACH_MEMORY").is_some_and(|v| !v.is_empty()) {
        // ⚠ Keep this literal in sync with `crates/vedge-e2e/tests/daily_loop.rs`
        // (`scan_health_shows_breached` seeds a login with this exact password).
        return Arc::new(
            vedge_core::infrastructure::breach::MemoryBreachChecker::with_breached(&[(
                "e2e-breached-pass-123",
                5,
            )]),
        );
    }
    Arc::new(HibpBreachChecker::new())
}

#[cfg(not(debug_assertions))]
fn resolve_breach() -> Arc<dyn BreachChecker> {
    Arc::new(HibpBreachChecker::new())
}

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
    let app_dir = resolve_app_dir(app)?;
    std::fs::create_dir_all(&app_dir)?;

    let app_db_path = app_dir.join("app.db");
    let app_conn = AppDbConnection::open(&app_db_path).await?;

    let recent_vaults: Arc<dyn RecentVaultRepository> =
        Arc::new(SqliteRecentVaultRepository::new(app_conn.handle()));
    let app_settings: Arc<dyn AppSettingRepository> =
        Arc::new(SqliteAppSettingRepository::new(app_conn.handle()));
    let themes: Arc<dyn ThemeRepository> = Arc::new(SqliteThemeRepository::new(app_conn.handle()));
    let known_devices: Arc<dyn KnownDeviceRepository> =
        Arc::new(SqliteKnownDeviceRepository::new(app_conn.handle()));
    let extension_sessions: Arc<dyn ExtensionSessionRepository> =
        Arc::new(SqliteExtensionSessionRepository::new(app_conn.handle()));

    let crypto: Arc<dyn CryptoProvider> = Arc::new(XChaCha20CryptoProvider::new());
    let kdf: Arc<dyn KeyDerivationProvider> = Arc::new(Argon2idKdfProvider::new());
    let keychain: Arc<dyn KeychainProvider> = Arc::new(OsKeychainProvider::new());
    let biometric: Arc<dyn BiometricAuthenticator> = resolve_biometric(Arc::clone(&crypto));
    let clipboard: Arc<dyn ClipboardProvider> = Arc::new(ArboardClipboardProvider::new()?);
    let breach: Arc<dyn BreachChecker> = resolve_breach();

    // Pre-wire the unlock use case so per-vault repo/blob construction is
    // the use case's job at `execute` time. The factories are cheap Arcs.
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

    Ok(AppState::new(
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
    use std::path::PathBuf;
    use vedge_core::domain::shared::VaultId;

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
        let keychain: Arc<dyn KeychainProvider> =
            Arc::new(vedge_core::infrastructure::keychain::MemoryKeychainProvider::new());
        // Real biometric hardware isn't reachable in tests — use the in-memory variant.
        let biometric: Arc<dyn BiometricAuthenticator> =
            Arc::new(vedge_core::infrastructure::biometric::MemoryBiometricAuthenticator::new());
        // Clipboard can fail in CI / headless — use the in-memory variant.
        let clipboard: Arc<dyn ClipboardProvider> =
            Arc::new(vedge_core::infrastructure::clipboard::MemoryClipboardProvider::new());
        // No network in tests — the no-op double never reports a breach.
        let breach: Arc<dyn BreachChecker> =
            Arc::new(vedge_core::infrastructure::breach::NoopBreachChecker::new());

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

    #[tokio::test]
    async fn compose_wires_every_port_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let state = compose_test_state(&dir).await;
        assert!(!state.is_unlocked(&VaultId::new(PathBuf::from("/tmp/does-not-exist.vdb"))));
    }
}
