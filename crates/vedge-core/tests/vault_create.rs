#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value,
    clippy::items_after_statements
)]

//! Integration tests for the `create_vault` use case. Uses the real crypto +
//! KDF + `SQLite` + blob providers (fast Argon2id params) against a `TempDir`,
//! and proves a created vault unlocks through the unchanged `UnlockVault`.

mod common;

use std::path::Path;
use std::sync::Arc;

use common::fast_kdf_params;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BlobStoreFactory, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository, VaultRepositoryFactory,
};
use vedge_core::application::vault::use_cases::{
    CreateVault, CreateVaultInput, UnlockVault, UnlockVaultInput,
};
use vedge_core::domain::shared::VaultId;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::recovery::parse_secret_key;
use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;
use vedge_core::infrastructure::crypto::{Argon2idKdfProvider, XChaCha20CryptoProvider};
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

/// App-lifetime providers shared between a `CreateVault` and a matching
/// `UnlockVault`. The keychain is shared so a `create` → `unlock` round-trip
/// can resolve the Secret Key without re-typing it.
struct Providers {
    crypto: Arc<XChaCha20CryptoProvider>,
    kdf: Arc<Argon2idKdfProvider>,
    keychain: Arc<MemoryKeychainProvider>,
    clipboard: Arc<MemoryClipboardProvider>,
}

fn providers() -> Providers {
    Providers {
        crypto: Arc::new(XChaCha20CryptoProvider::new()),
        kdf: Arc::new(Argon2idKdfProvider::new()),
        keychain: Arc::new(MemoryKeychainProvider::new()),
        clipboard: Arc::new(MemoryClipboardProvider::new()),
    }
}

fn build_create(p: &Providers, keychain: Arc<dyn KeychainProvider>) -> CreateVault {
    CreateVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::clone(&p.crypto) as Arc<dyn CryptoProvider>,
        clipboard: Arc::clone(&p.clipboard) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&p.kdf) as Arc<dyn KeyDerivationProvider>,
        keychain,
    }
}

fn build_unlock(p: &Providers) -> UnlockVault {
    UnlockVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::clone(&p.crypto) as Arc<dyn CryptoProvider>,
        clipboard: Arc::clone(&p.clipboard) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&p.kdf) as Arc<dyn KeyDerivationProvider>,
        keychain: Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>,
    }
}

fn open_repo(path: &Path) -> impl std::future::Future<Output = Arc<dyn VaultRepository>> {
    let path = path.to_path_buf();
    async move {
        SqliteVaultRepositoryFactory::new()
            .open(&path)
            .await
            .unwrap()
    }
}

fn make_input(path: &Path) -> CreateVaultInput {
    CreateVaultInput {
        vault_path: path.to_path_buf(),
        master_password: Zeroizing::new("correct horse battery staple".into()),
        secret_key: None,
        kdf_params: Some(fast_kdf_params()),
    }
}

#[tokio::test]
async fn create_then_unlock_roundtrip() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    let out = create.execute(make_input(&path)).await.unwrap();
    assert!(out.keychain_stored);
    assert!(out.keychain_error.is_none());
    assert!(
        out.session.index().all_active().is_empty(),
        "fresh vault index must be empty"
    );
    // Close the creation session so unlock opens a clean connection.
    drop(out.session);

    // Unlock with the same password; the Secret Key comes from the keychain.
    let unlock = build_unlock(&p);
    let session = unlock
        .execute(UnlockVaultInput {
            vault_path: path.clone(),
            master_password: Zeroizing::new("correct horse battery staple".into()),
            secret_key: None,
        })
        .await
        .unwrap();
    assert!(session.index().all_active().is_empty());
    assert_eq!(session.vault_id(), &VaultId::new(path));
}

#[tokio::test]
async fn create_refuses_existing_path() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    drop(create.execute(make_input(&path)).await.unwrap().session);

    let err = create.execute(make_input(&path)).await.unwrap_err();
    assert!(matches!(err, VaultError::VaultAlreadyExists));
}

#[tokio::test]
async fn unlock_with_wrong_password_fails() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    drop(create.execute(make_input(&path)).await.unwrap().session);

    let unlock = build_unlock(&p);
    let err = unlock
        .execute(UnlockVaultInput {
            vault_path: path,
            master_password: Zeroizing::new("WRONG password".into()),
            secret_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials));
}

#[tokio::test]
async fn secret_key_display_roundtrips() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    // Pin an explicit Secret Key so we know the expected bytes.
    let raw = [0xABu8; SECRET_KEY_LEN];
    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    let out = create
        .execute(CreateVaultInput {
            vault_path: path.clone(),
            master_password: Zeroizing::new("correct horse battery staple".into()),
            secret_key: Some(Zeroizing::new(raw)),
            kdf_params: Some(fast_kdf_params()),
        })
        .await
        .unwrap();

    let parsed = parse_secret_key(&out.secret_key_display).unwrap();
    assert_eq!(*parsed, raw);
}

#[tokio::test]
async fn config_fields_are_correct() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    drop(create.execute(make_input(&path)).await.unwrap().session);

    let repo = open_repo(&path).await;
    let config = repo.load_config().await.unwrap();
    assert_eq!(config.magic, "VEDG");
    assert_eq!(config.id, "default");
    assert_eq!(config.schema_version, 1);
    assert_eq!(config.preferred_cipher_suite, 1);
    assert_eq!(config.kdf_params, fast_kdf_params());
    assert!(config.last_unlocked_at.is_some());
    // Slice 4.6a: default audit retention now matches the migration + doc 20.2 (was 365).
    assert_eq!(config.audit_retention_days, 90);
    // Intrinsic identity minted at create.
    assert!(config.vault_uuid.is_some(), "create must mint a vault_uuid");
}

#[tokio::test]
async fn vault_uuid_minted_and_stable_across_unlocks() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    drop(create.execute(make_input(&path)).await.unwrap().session);

    let uuid1 = open_repo(&path)
        .await
        .load_config()
        .await
        .unwrap()
        .vault_uuid;
    assert!(uuid1.is_some(), "create must mint a vault_uuid");

    // Two unlocks must not disturb the already-present identity.
    let unlock = build_unlock(&p);
    for _ in 0..2 {
        drop(
            unlock
                .execute(UnlockVaultInput {
                    vault_path: path.clone(),
                    master_password: Zeroizing::new("correct horse battery staple".into()),
                    secret_key: None,
                })
                .await
                .unwrap(),
        );
    }

    let uuid2 = open_repo(&path)
        .await
        .load_config()
        .await
        .unwrap()
        .vault_uuid;
    assert_eq!(uuid1, uuid2, "vault_uuid must be stable across unlocks");
}

#[tokio::test]
async fn created_audit_event_appended() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    let create = build_create(&p, Arc::clone(&p.keychain) as Arc<dyn KeychainProvider>);
    drop(create.execute(make_input(&path)).await.unwrap().session);

    let repo = open_repo(&path).await;
    let recent = repo.recent_audit(10).await.unwrap();
    let created_count = recent
        .iter()
        .filter(|e| matches!(e.action, AuditAction::Created))
        .count();
    assert_eq!(created_count, 1, "exactly one Created audit event expected");
}

#[tokio::test]
async fn keychain_failure_returns_valid_vault() {
    let p = providers();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.vedge");

    // A keychain whose store always fails — the vault must still be valid.
    struct FailingKeychain;
    impl KeychainProvider for FailingKeychain {
        fn read_secret_key(
            &self,
            _vault_uuid: &str,
        ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn store_secret_key(
            &self,
            _vault_uuid: &str,
            _key: &[u8; SECRET_KEY_LEN],
        ) -> Result<(), VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn delete_secret_key(&self, _vault_uuid: &str) -> Result<(), VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn read_commit_baseline(&self, _vault_uuid: &str) -> Result<Option<i64>, VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn store_commit_baseline(
            &self,
            _vault_uuid: &str,
            _counter: i64,
        ) -> Result<(), VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn delete_commit_baseline(&self, _vault_uuid: &str) -> Result<(), VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn migrate_secret_key(
            &self,
            _legacy_vault_id: &VaultId,
            _vault_uuid: &str,
        ) -> Result<bool, VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
    }

    let create = build_create(&p, Arc::new(FailingKeychain));
    let out = create.execute(make_input(&path)).await.unwrap();

    assert!(!out.keychain_stored, "keychain store was rigged to fail");
    assert!(out.keychain_error.is_some());
    // Session is still valid + unlocked.
    assert_eq!(out.session.vault_id(), &VaultId::new(path.clone()));
    assert!(out.session.index().all_active().is_empty());
    drop(out.session);

    // The on-disk vault is valid: config + Created audit persisted.
    let repo = open_repo(&path).await;
    assert_eq!(repo.load_config().await.unwrap().magic, "VEDG");
    let recent = repo.recent_audit(10).await.unwrap();
    assert!(
        recent
            .iter()
            .any(|e| matches!(e.action, AuditAction::Created))
    );
}
