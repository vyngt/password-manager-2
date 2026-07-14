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
use vedge_core::application::vault::ports::{
    BiometricAuthenticator, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, UnlockVaultInput, change_password, create_entry,
    create_snapshot, lock_vault,
};
use vedge_core::domain::shared::SNAPSHOTS_DIR;
use vedge_core::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::snapshot::manifest::{SnapshotReason, verify_hash_prefix};
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

async fn unlock(h: &Harness, pw: &str) -> Result<VaultSession, VaultError> {
    let uv = build_unlock(h);
    uv.execute(UnlockVaultInput {
        vault_path: h.home.clone(),
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
        totp_params: vedge_core::TotpParams::default(),
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
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
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
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
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
    let read_sk = h.keychain.read_secret_key(&h.vault_uuid).unwrap();
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
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
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

/// Slice 4.6a: change-password persists a rebuilt config through `rewrap_all_deks`'s
/// OWN upsert (distinct from `save_config`). The intrinsic `vault_uuid` must survive
/// that path — a regression that dropped it from the rewrap upsert would fail here.
#[tokio::test]
async fn vault_uuid_survives_change_password() {
    let h = Harness::fresh().await;
    // The harness vault carries an intrinsic uuid (slice 5.2.0); the point here is that
    // change_password preserves it through the `rewrap_all_deks` upsert.
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    let uuid_before = h.repo.load_config().await.unwrap().vault_uuid;
    assert!(uuid_before.is_some(), "vault must carry a uuid");

    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new("next-pw".into()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    assert_eq!(
        h.repo.load_config().await.unwrap().vault_uuid,
        uuid_before,
        "change_password must preserve vault_uuid"
    );
}

// ---- slice 5.2.1: snapshot rewrap on credential change (Decision ⑬ / §B) ----

/// Derive the `(kek, verify_hash)` a given password + the harness Secret Key would produce —
/// so tests can check a snapshot's `vault.vdb` opens under the NEW credentials.
fn derive_kek_and_verify(h: &Harness, pw: &str) -> (Zeroizing<[u8; KEK_LEN]>, [u8; 32]) {
    let input = h.kdf.preprocess_2skd(pw.as_bytes(), &h.secret_key).unwrap();
    let mk = h
        .kdf
        .derive_master_key(&input, &h.config.vault_salt, &h.config.kdf_params)
        .unwrap();
    let verify = h.kdf.derive_verify_hash(&mk).unwrap();
    let kek = h.kdf.derive_kek(&mk).unwrap();
    (kek, verify)
}

async fn change_pw(session: &mut VaultSession, h: &Harness, new_pw: &str) {
    change_password(
        session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new(new_pw.to_owned()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
}

/// The single pooled object's bytes (there is exactly one blob in these tests).
fn only_object_bytes(h: &Harness) -> Vec<u8> {
    let objects = h.home.join(SNAPSHOTS_DIR).join("objects");
    let mut files: Vec<_> = std::fs::read_dir(&objects)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    std::fs::read(&files[0]).unwrap()
}

/// Test 9 (⑬): after a password change a snapshot opens with the NEW password — its
/// `verify_hash` matches, a snapshot DEK unwraps under the new KEK, the object pool is
/// byte-identical (blobs never touched), and the manifest's `verify_hash_prefix` is updated.
#[tokio::test]
async fn snapshot_rewraps_to_the_new_credentials() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("e1", "p1"),
        },
    )
    .await
    .unwrap();
    // Seed a blob so the pool has an object to prove untouched.
    std::fs::write(
        h.blob.root().join(format!(
            "{}.blob",
            vedge_core::domain::shared::EntryId::new()
        )),
        b"blob nonce+ciphertext",
    )
    .unwrap();
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    let objects_before = only_object_bytes(&h);
    let store_dir = h.home.join(SNAPSHOTS_DIR);
    let snap_dir = store::list_snapshots(&store_dir).unwrap()[0].dir.clone();

    change_pw(&mut session, &h, "brand-new-password").await;
    lock_vault(session).await.unwrap();

    let (new_kek, new_verify) = derive_kek_and_verify(&h, "brand-new-password");

    // (a) The snapshot's own config now carries the NEW verify_hash, and a snapshot DEK
    //     unwraps under the NEW KEK.
    let db = VaultDbConnection::open(&snap_dir.join("vault.vdb"))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(
        repo.load_config().await.unwrap().verify_hash,
        new_verify,
        "snapshot verify_hash rewrapped to the new credentials"
    );
    let rows = repo.all_entries().await.unwrap();
    assert!(!rows.is_empty());
    h.crypto
        .unwrap_dek(&rows[0].dek_wrapped, &new_kek)
        .expect("snapshot DEK must unwrap under the NEW KEK");
    drop(repo);
    db.close().await.unwrap();

    // (b) The object pool is byte-identical — a rewrap never touches blobs.
    assert_eq!(
        only_object_bytes(&h),
        objects_before,
        "objects untouched by rewrap"
    );

    // (c) The manifest's verify_hash_prefix reflects the new credentials.
    let manifest = store::read_manifest(&snap_dir).unwrap();
    assert_eq!(manifest.verify_hash_prefix, verify_hash_prefix(&new_verify));
}

/// Test 10 (⑬): a snapshot that CANNOT be rewrapped (its `vault.vdb` is corrupt) does NOT
/// fail the credential change — the live vault commits, unlocks with the new password, and
/// the good snapshot is still rewrapped.
#[tokio::test]
async fn a_failed_snapshot_rewrap_never_fails_the_change() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("e1", "p1"),
        },
    )
    .await
    .unwrap();
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    let store_dir = h.home.join(SNAPSHOTS_DIR);
    let snaps = store::list_snapshots(&store_dir).unwrap();
    assert_eq!(snaps.len(), 2);
    // Corrupt the NEWEST snapshot's vault.vdb so its rewrap fails.
    std::fs::write(snaps[0].dir.join("vault.vdb"), b"not a database").unwrap();
    let good_dir = snaps[1].dir.clone();

    // The change must still succeed despite the corrupt snapshot.
    change_pw(&mut session, &h, "new-password-2").await;
    lock_vault(session).await.unwrap();

    // Live vault opens with the new password.
    let s = unlock(&h, "new-password-2").await.unwrap();
    lock_vault(s).await.unwrap();

    // The GOOD snapshot was still rewrapped to the new credentials.
    let (_kek, new_verify) = derive_kek_and_verify(&h, "new-password-2");
    let db = VaultDbConnection::open(&good_dir.join("vault.vdb"))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(repo.load_config().await.unwrap().verify_hash, new_verify);
    drop(repo);
    db.close().await.unwrap();
}
