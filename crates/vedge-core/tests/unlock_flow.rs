#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use std::sync::Arc;

use common::Harness;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BlobStore, ClipboardProvider, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository,
};
// The imports above may look unused in parts of the test but they're needed
// for the explicit trait-object coercions in `build_unlock`.
use vedge_core::application::vault::use_cases::{lock_vault, UnlockVault, UnlockVaultInput};
use vedge_core::domain::vault::errors::VaultError;

fn build_unlock(h: &Harness) -> UnlockVault {
    UnlockVault {
        repo: Arc::clone(&h.repo) as Arc<dyn VaultRepository>,
        crypto: Arc::clone(&h.crypto) as Arc<dyn CryptoProvider>,
        blob: Arc::clone(&h.blob) as Arc<dyn BlobStore>,
        clipboard: Arc::clone(&h.clipboard) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        keychain: Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
    }
}

#[tokio::test]
async fn unlock_returns_session_with_seeded_index() {
    let h = Harness::fresh().await;
    let _id = h.seed_login("github", "alice", "hunter2").await;
    let _tag_id = h.seed_tag("github").await;

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None, // read from keychain
        })
        .await
        .unwrap();

    let active = session.index().all_active();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "github");
    assert_eq!(session.index().tags.len(), 1);

    // Explicit lock records the audit event and drops the session.
    lock_vault(session).await.unwrap();
}

#[tokio::test]
async fn wrong_password_fast_rejects_before_index_build() {
    let h = Harness::fresh().await;
    h.seed_login("github", "alice", "hunter2").await;

    let uv = build_unlock(&h);
    let err = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: Zeroizing::new("WRONG password".into()),
            secret_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials));
}

#[tokio::test]
async fn missing_secret_key_errors() {
    let h = Harness::fresh().await;
    // Deliberately remove the key from the memory keychain.
    h.keychain.delete_secret_key(&h.vault_id).unwrap();

    let uv = build_unlock(&h);
    let err = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::KeychainEntryNotFound));
}

#[tokio::test]
async fn tampered_verify_hash_becomes_wrong_credentials() {
    let h = Harness::fresh().await;

    // Flip the stored verify_hash behind UnlockVault's back.
    let mut config = h.repo.load_config().await.unwrap();
    config.verify_hash[0] ^= 0xFF;
    h.repo.save_config(&config).await.unwrap();

    let uv = build_unlock(&h);
    let err = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials));
}

#[tokio::test]
async fn explicit_secret_key_bypasses_keychain() {
    let h = Harness::fresh().await;
    // Even with the key missing from the keychain, caller-supplied input works.
    h.keychain.delete_secret_key(&h.vault_id).unwrap();

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: Some(Zeroizing::new(h.secret_key)),
        })
        .await
        .unwrap();
    assert!(session.index().all_active().is_empty());
}

#[tokio::test]
async fn last_unlocked_at_is_updated() {
    let h = Harness::fresh().await;
    assert!(h.config.last_unlocked_at.is_none());

    let uv = build_unlock(&h);
    let session = uv
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();
    drop(session); // release the DB lock

    let reloaded = h.repo.load_config().await.unwrap();
    assert!(reloaded.last_unlocked_at.is_some());
}
