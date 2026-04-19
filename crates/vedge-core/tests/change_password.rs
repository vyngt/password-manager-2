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

use common::{Harness, build_unlock};
use secrecy::SecretString;
use zeroize::Zeroizing;

// Traits needed for trait-method lookup + trait-object coercions.
use vedge_core::application::vault::ports::{KeyDerivationProvider, KeychainProvider};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, UnlockVaultInput, change_password, create_entry,
    lock_vault,
};
use vedge_core::domain::vault::crypto_constants::SECRET_KEY_LEN;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};

async fn unlock(h: &Harness, pw: &str) -> Result<VaultSession, VaultError> {
    let uv = build_unlock(h);
    uv.execute(UnlockVaultInput {
        vault_path: h.vdb_path.clone(),
        master_password: Zeroizing::new(pw.to_owned()),
        secret_key: None,
    })
    .await
}

fn login(name: &str, pw: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "alice".into(),
        password: SecretString::from(pw),
        totp_secret: None,
        recovery_codes: vec![],
    })
}

#[tokio::test]
async fn rotate_password_then_unlock_with_new_only() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();

    // Seed 3 entries so we exercise the n-way rewrap.
    for i in 0..3 {
        create_entry(
            &mut session,
            CreateEntryInput {
                payload: login(&format!("gh-{i}"), "old-pw"),
            },
        )
        .await
        .unwrap();
    }

    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ChangePasswordInput {
            new_password: Zeroizing::new("new-hunter2".into()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    // Unlock with new succeeds.
    let session_new = unlock(&h, "new-hunter2").await.unwrap();
    assert_eq!(session_new.index().all_active().len(), 3);
    lock_vault(session_new).await.unwrap();

    // Unlock with old fails.
    let err = unlock(&h, "correct horse battery staple")
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials));
}

#[tokio::test]
async fn rotate_secret_key_only() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();

    let new_sk: [u8; SECRET_KEY_LEN] = [0xEE; SECRET_KEY_LEN];
    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ChangePasswordInput {
            new_password: Zeroizing::new("correct horse battery staple".into()),
            new_secret_key: Some(Zeroizing::new(new_sk)),
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    // Keychain has the new Secret Key; unlocking with the original password
    // still works because it now pairs with the new SK via the keychain.
    let session_new = unlock(&h, "correct horse battery staple").await.unwrap();
    lock_vault(session_new).await.unwrap();

    // Confirm the keychain actually got rewritten.
    let read_sk = h.keychain.read_secret_key(&h.vault_id).unwrap();
    assert_eq!(*read_sk, new_sk);
}

#[tokio::test]
async fn rotate_preserves_all_existing_entries_decrypted() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();

    // Remember the set of entry IDs we seeded.
    let mut expected_ids = Vec::new();
    for i in 0..5 {
        expected_ids.push(
            create_entry(
                &mut session,
                CreateEntryInput {
                    payload: login(&format!("e{i}"), "pw"),
                },
            )
            .await
            .unwrap()
            .entry_id,
        );
    }
    expected_ids.sort_by_key(|id| id.as_str().to_owned());

    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        ChangePasswordInput {
            new_password: Zeroizing::new("next-pw".into()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    let session_new = unlock(&h, "next-pw").await.unwrap();
    let mut got: Vec<_> = session_new.index().entries.keys().cloned().collect();
    got.sort_by_key(|id| id.as_str().to_owned());
    assert_eq!(got, expected_ids);
    lock_vault(session_new).await.unwrap();
}
