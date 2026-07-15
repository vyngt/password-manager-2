//! Rollback-detection host suite (slice 5.2c): the unlock-time compare of the
//! vault's `commit_counter` against this device's keychain baseline.
//!
//! The warning is advisory (Decision: warning, never an error). It must fire on a
//! genuine rollback, stay silent for a fresh device and for a device that's merely
//! behind, and it must NEVER fail an unlock — not even when the keychain errors.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

mod common;

use std::sync::Arc;

use zeroize::Zeroizing;

use common::{Harness, build_unlock};
use vedge_core::application::vault::ports::{
    BlobStoreFactory, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository, VaultRepositoryFactory,
};
use vedge_core::application::vault::use_cases::{UnlockVault, UnlockVaultInput};
use vedge_core::domain::shared::VaultId;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

fn pw_input(h: &Harness) -> UnlockVaultInput {
    UnlockVaultInput {
        vault_path: h.home.clone(),
        master_password: h.master_password.clone(),
        secret_key: None,
    }
}

async fn vault_uuid(h: &Harness) -> String {
    h.repo.load_config().await.unwrap().vault_uuid.unwrap()
}

async fn has_rollback_audit(h: &Harness) -> bool {
    h.repo
        .recent_audit(50)
        .await
        .unwrap()
        .iter()
        .any(|e| e.action == AuditAction::RollbackDetected)
}

#[tokio::test]
async fn rollback_warning_fires_only_when_file_is_behind_the_baseline() {
    let h = Harness::fresh().await;
    let unlock = build_unlock(&h);

    // (1) FRESH DEVICE — the first unlock mints the uuid and establishes the baseline
    //     at the current file counter (0). No baseline existed → no warning.
    let s1 = unlock.execute(pw_input(&h)).await.unwrap();
    assert_eq!(s1.rollback_warning(), None, "fresh device must not warn");
    drop(s1);
    let uuid = vault_uuid(&h).await;
    assert_eq!(
        h.keychain.read_commit_baseline(&uuid).unwrap(),
        Some(0),
        "fresh unlock establishes the baseline at the file counter"
    );
    assert!(!has_rollback_audit(&h).await);

    // (2) DEVICE BEHIND — advance the file past the baseline (3 content writes → 3),
    //     then unlock: file(3) > baseline(0) → no warn, and the mirror advances to 3.
    h.seed_login("a", "u", "p").await;
    h.seed_login("b", "u", "p").await;
    h.seed_login("c", "u", "p").await;
    let s2 = unlock.execute(pw_input(&h)).await.unwrap();
    assert_eq!(
        s2.rollback_warning(),
        None,
        "ahead-of-baseline must not warn"
    );
    drop(s2);
    assert_eq!(
        h.keychain.read_commit_baseline(&uuid).unwrap(),
        Some(3),
        "a device that's behind advances the mirror to the file counter"
    );
    assert!(!has_rollback_audit(&h).await);

    // (3) ROLLBACK — this device once saw state 10, but the file is only at 3 (an old
    //     snapshot was swapped in): file(3) < baseline(10) → warn Some(7), mirror UNCHANGED.
    h.keychain.store_commit_baseline(&uuid, 10).unwrap();
    let s3 = unlock.execute(pw_input(&h)).await.unwrap();
    assert_eq!(
        s3.rollback_warning(),
        Some(7),
        "rollback must warn with the (baseline − file) delta"
    );
    drop(s3);
    assert_eq!(
        h.keychain.read_commit_baseline(&uuid).unwrap(),
        Some(10),
        "a rollback must NOT lower the baseline"
    );
    assert!(
        has_rollback_audit(&h).await,
        "a detected rollback writes one RollbackDetected audit row"
    );

    // (4) The biometric path runs the SAME compare: still file(3) < baseline(10).
    let s4 = unlock
        .unlock_with_kek(h.home.clone(), Zeroizing::new(h.kek))
        .await
        .unwrap();
    assert_eq!(
        s4.rollback_warning(),
        Some(7),
        "biometric unlock warns on a rollback too"
    );
}

/// R6 safety property: a keychain that fails the baseline read must neither warn nor
/// fail the unlock — a broken mirror must never manufacture a false alarm or brick a
/// vault.
#[tokio::test]
async fn rollback_check_never_fails_unlock_when_keychain_read_errors() {
    /// Delegates the Secret-Key methods to a working memory keychain but always errors
    /// the baseline read.
    struct BaselineReadFails {
        inner: Arc<MemoryKeychainProvider>,
    }
    impl KeychainProvider for BaselineReadFails {
        fn read_secret_key(
            &self,
            vault_uuid: &str,
        ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
            self.inner.read_secret_key(vault_uuid)
        }
        fn store_secret_key(
            &self,
            vault_uuid: &str,
            key: &[u8; SECRET_KEY_LEN],
        ) -> Result<(), VaultError> {
            self.inner.store_secret_key(vault_uuid, key)
        }
        fn delete_secret_key(&self, vault_uuid: &str) -> Result<(), VaultError> {
            self.inner.delete_secret_key(vault_uuid)
        }
        fn read_commit_baseline(&self, _vault_uuid: &str) -> Result<Option<i64>, VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn store_commit_baseline(&self, vault_uuid: &str, counter: i64) -> Result<(), VaultError> {
            self.inner.store_commit_baseline(vault_uuid, counter)
        }
        fn delete_commit_baseline(&self, vault_uuid: &str) -> Result<(), VaultError> {
            self.inner.delete_commit_baseline(vault_uuid)
        }
        fn migrate_secret_key(
            &self,
            legacy_vault_id: &VaultId,
            vault_uuid: &str,
        ) -> Result<bool, VaultError> {
            self.inner.migrate_secret_key(legacy_vault_id, vault_uuid)
        }
    }

    let h = Harness::fresh().await;
    let unlock = UnlockVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::clone(&h.crypto) as Arc<dyn CryptoProvider>,
        clipboard: Arc::clone(&h.clipboard) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        keychain: Arc::new(BaselineReadFails {
            inner: Arc::clone(&h.keychain),
        }) as Arc<dyn KeychainProvider>,
    };

    let session = unlock.execute(pw_input(&h)).await.unwrap();
    assert_eq!(
        session.rollback_warning(),
        None,
        "a keychain read error must not warn and must not fail the unlock"
    );
}
