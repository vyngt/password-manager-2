#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value,
    clippy::items_after_statements
)]

mod common;

use std::sync::Arc;

use common::{Harness, build_unlock};
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{KeychainProvider, VaultRepository};
use vedge_core::application::vault::use_cases::{RecoverVaultInput, recover_vault};
use vedge_core::domain::shared::VaultId;
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::recovery::format_secret_key;

#[tokio::test]
async fn recover_vault_happy_path_restores_keychain() {
    let h = Harness::fresh().await;
    let display = format_secret_key(&h.secret_key);

    // Simulate the machine where DPAPI lost the key.
    h.keychain.delete_secret_key(&h.vault_id).unwrap();

    let uv = build_unlock(&h);
    let outcome = recover_vault(
        &uv,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap();

    assert!(outcome.keychain_restored, "keychain should be restored");
    assert!(outcome.keychain_error.is_none());

    // Key was re-written so subsequent unlocks should find it.
    let restored = h.keychain.read_secret_key(&h.vault_id).unwrap();
    assert_eq!(*restored, h.secret_key);

    // Session is live.
    assert_eq!(outcome.session.vault_id(), &h.vault_id);
}

#[tokio::test]
async fn recover_vault_rejects_invalid_checksum_fast() {
    let h = Harness::fresh().await;
    let mut display = format_secret_key(&h.secret_key);
    // Flip one char in the body (skip prefix) to break checksum.
    display.replace_range(3..=3, "Z");

    let uv = build_unlock(&h);
    let err = recover_vault(
        &uv,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, VaultError::InvalidRecoveryKey(_)));
}

#[tokio::test]
async fn recover_vault_wrong_password_does_not_touch_keychain() {
    let h = Harness::fresh().await;
    let display = format_secret_key(&h.secret_key);

    // Pre-state: delete the keychain entry. If recovery mistakenly restores
    // it on a failed unlock, the assertion at the end will notice.
    h.keychain.delete_secret_key(&h.vault_id).unwrap();

    let uv = build_unlock(&h);
    let err = recover_vault(
        &uv,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: Zeroizing::new("WRONG password".into()),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(err, VaultError::WrongCredentials));
    // Keychain must still be empty — we do not eagerly restore before unlock succeeds.
    let probe = h.keychain.read_secret_key(&h.vault_id);
    assert!(matches!(probe, Err(VaultError::KeychainEntryNotFound)));
}

#[tokio::test]
async fn recover_vault_emits_recovery_used_audit_event() {
    let h = Harness::fresh().await;
    let display = format_secret_key(&h.secret_key);
    h.keychain.delete_secret_key(&h.vault_id).unwrap();

    let uv = build_unlock(&h);
    let _outcome = recover_vault(
        &uv,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap();

    let recent = h.repo.recent_audit(10).await.unwrap();
    let has_recovery = recent
        .iter()
        .any(|e| matches!(e.action, AuditAction::RecoveryUsed));
    let has_unlock = recent
        .iter()
        .any(|e| matches!(e.action, AuditAction::Unlocked));
    assert!(has_recovery, "RecoveryUsed audit event must be appended");
    assert!(has_unlock, "UnlockVault's Unlocked event must still fire");
}

#[tokio::test]
async fn recover_vault_surfaces_partial_outcome_when_keychain_write_fails() {
    // Use a failing-on-store mock keychain to exercise the partial path.
    let h = Harness::fresh().await;
    let display = format_secret_key(&h.secret_key);

    struct FailingStoreKeychain {
        inner: Arc<vedge_core::infrastructure::keychain::MemoryKeychainProvider>,
    }

    impl KeychainProvider for FailingStoreKeychain {
        fn read_secret_key(
            &self,
            vault_id: &VaultId,
        ) -> Result<Zeroizing<[u8; SECRET_KEY_LEN]>, VaultError> {
            self.inner.read_secret_key(vault_id)
        }
        fn store_secret_key(
            &self,
            _vault_id: &VaultId,
            _key: &[u8; SECRET_KEY_LEN],
        ) -> Result<(), VaultError> {
            Err(VaultError::KeychainUnavailable)
        }
        fn delete_secret_key(&self, vault_id: &VaultId) -> Result<(), VaultError> {
            self.inner.delete_secret_key(vault_id)
        }
        fn read_commit_baseline(&self, vault_uuid: &str) -> Result<Option<i64>, VaultError> {
            self.inner.read_commit_baseline(vault_uuid)
        }
        fn store_commit_baseline(&self, vault_uuid: &str, counter: i64) -> Result<(), VaultError> {
            self.inner.store_commit_baseline(vault_uuid, counter)
        }
    }

    let failing: Arc<dyn KeychainProvider> = Arc::new(FailingStoreKeychain {
        inner: Arc::clone(&h.keychain),
    });

    let uv = build_unlock(&h);
    let outcome = recover_vault(
        &uv,
        Arc::clone(&failing),
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap();

    assert!(
        !outcome.keychain_restored,
        "keychain write failed, so restored should be false"
    );
    assert!(
        outcome.keychain_error.is_some(),
        "error message must be surfaced"
    );
}

/// Slice 4.6a: `recover_vault` delegates to `UnlockVault::execute`, so it inherits
/// the `vault_uuid` backfill on first open of a pre-4.6 vault.
#[tokio::test]
async fn vault_uuid_backfilled_by_recover_vault() {
    let h = Harness::fresh().await;
    let display = format_secret_key(&h.secret_key);
    assert!(
        h.repo.load_config().await.unwrap().vault_uuid.is_none(),
        "precondition: a pre-4.6 vault has no uuid"
    );

    let uv = build_unlock(&h);
    let outcome = recover_vault(
        &uv,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        RecoverVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            recovery_key_display: display,
        },
    )
    .await
    .unwrap();
    drop(outcome.session);

    assert!(
        h.repo.load_config().await.unwrap().vault_uuid.is_some(),
        "recover_vault must backfill vault_uuid"
    );
}
