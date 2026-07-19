//! Slice 5.6.0 — Tag Keys.
//!
//! Tags used to be sealed directly under the vault KEK (no per-row DEK), so a
//! master-password change re-wrapped every entry DEK + `verify_hash` but left the
//! tag rows under the OLD KEK — bricking any tagged vault (neither password opens
//! it) and, via ⑬, every snapshot of it. This slice gives tags a per-row DEK and
//! deletes the ability to seal a tag under the KEK. These tests reproduce the brick
//! first, then prove the fix, the at-unlock migration, and the snapshot rewrap.

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

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    ChangePasswordInput, CreateEntryInput, UnlockVaultInput, change_password, create_entry,
    create_snapshot, create_tag, lock_vault,
};
use vedge_core::domain::shared::{SNAPSHOTS_DIR, TagId};
use vedge_core::domain::vault::aad::tag_aad;
use vedge_core::domain::vault::crypto_constants::{KEK_LEN, SECRET_KEY_LEN};
use vedge_core::domain::vault::entities::TagRow;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, LoginPayload, TagPayload,
};
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

// ---- helpers ------------------------------------------------------------

async fn unlock(h: &Harness, pw: &str) -> Result<VaultSession, VaultError> {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: Zeroizing::new(pw.to_owned()),
            secret_key: None,
        })
        .await
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
            secret_key_rotated: false,
        },
    )
    .await
    .unwrap();
}

fn login_tagged(name: &str, tag_ids: Vec<TagId>) -> EntryPayload {
    let mut meta = CommonMeta::new(name, EntryType::Login);
    meta.tag_ids = tag_ids;
    EntryPayload::Login(LoginPayload {
        meta,
        username: "alice".into(),
        password: SecretString::from("pw"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

/// Derive the KEK a `(password, harness Secret Key)` pair produces, so a test can read a
/// snapshot's tag under the new credentials (mirror of the `change_password` test helper).
fn derive_kek(h: &Harness, pw: &str) -> Zeroizing<[u8; KEK_LEN]> {
    let input = h.kdf.preprocess_2skd(pw.as_bytes(), &h.secret_key).unwrap();
    let mk = h
        .kdf
        .derive_master_key(&input, &h.config.vault_salt, &h.config.kdf_params)
        .unwrap();
    h.kdf.derive_kek(&mk).unwrap()
}

/// Test-side mirror of `tag_crypto::open_tag_row` (which is `pub(crate)`): decrypt a
/// DEK-sealed tag row under `kek` and return its name. Panics on a legacy NULL row —
/// the tests that use it assert the row was migrated to DEK-sealed.
fn read_dek_tag_name(h: &Harness, kek: &[u8; KEK_LEN], row: &TagRow) -> String {
    let aad = tag_aad(&row.id).unwrap();
    let wrapped = row
        .dek_wrapped
        .expect("expected a DEK-sealed tag, found a legacy NULL row");
    let dek = h.crypto.unwrap_dek(&wrapped, kek).unwrap();
    let pt = h
        .crypto
        .decrypt_entry(&dek, &row.nonce, &row.ciphertext, &aad)
        .unwrap();
    serde_json::from_slice::<TagPayload>(&pt).unwrap().name
}

// ---- tests --------------------------------------------------------------

/// 🔴 Test 1 — THE BUG, reproduced then fixed. A vault with a tag survives a password
/// change: the new password opens it and the tag is still readable. Written against the
/// pre-fix code this FAILS at `unlock` (the old-KEK tag can't be read under `KEK_new`).
#[tokio::test]
async fn tagged_vault_survives_password_change() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    let tag_id = create_tag(&mut session, "GitHub", None).await.unwrap();
    change_pw(&mut session, &h, "new-hunter2").await;
    lock_vault(session).await.unwrap();

    let session_new = unlock(&h, "new-hunter2")
        .await
        .expect("the new password must open the tagged vault");
    assert_eq!(
        session_new.index().tags.get(&tag_id).unwrap().name,
        "github"
    );
    lock_vault(session_new).await.unwrap();
}

/// Test 3 — the Secret-Key rotation path bricks the same way if tags are skipped. Rotate
/// the SK (keeping the password), then the original password still opens the vault (it now
/// pairs with the new SK via the keychain) and the tag is intact.
#[tokio::test]
async fn tagged_vault_survives_secret_key_rotation() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    let tag_id = create_tag(&mut session, "GitHub", None).await.unwrap();

    change_password(
        &mut session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new("correct horse battery staple".into()),
            new_secret_key: Some(Zeroizing::new([0xEE; SECRET_KEY_LEN])),
            secret_key_rotated: true,
        },
    )
    .await
    .unwrap();
    lock_vault(session).await.unwrap();

    let s = unlock(&h, "correct horse battery staple").await.unwrap();
    assert_eq!(s.index().tags.get(&tag_id).unwrap().name, "github");
    lock_vault(s).await.unwrap();
}

/// 🔴 Retirement guard (slice 5.9 ②). The legacy KEK-as-AEAD read path — `decrypt_legacy_tag` and
/// the at-unlock `migrate_legacy_tag` — was RETIRED behind a prove-absence audit
/// (`mise tag-legacy-audit`). So a pre-5.6.0 KEK-sealed tag (`dek_wrapped = None`) no longer
/// migrates on unlock; it HARD-FAILS (`DecryptionFailed`). Unreachable for a vault whose tags
/// migrated before the audit went green — this pins that the door is gone, not just unused.
#[tokio::test]
async fn retired_legacy_tag_hard_fails_unlock() {
    let h = Harness::fresh().await;
    h.seed_legacy_tag("orphan").await;

    let err = unlock(&h, "correct horse battery staple")
        .await
        .expect_err("a retired legacy tag must fail unlock, not silently migrate");
    assert!(
        matches!(err, VaultError::DecryptionFailed),
        "expected DecryptionFailed for a retired legacy tag, got {err:?}"
    );

    // And it was NOT rewritten — no silent migration behind the failure.
    assert!(
        h.repo.all_tags().await.unwrap()[0].dek_wrapped.is_none(),
        "the legacy tag must stay untouched (no partial migration write)"
    );
}

/// Test 2 — a snapshot's tags survive a password change: the snapshot's tag re-wraps in ⑬
/// and reads under the NEW KEK.
#[tokio::test]
async fn snapshot_tags_survive_password_change() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h, "correct horse battery staple").await.unwrap();
    let tag_id = create_tag(&mut session, "github", None).await.unwrap();
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_tagged("e1", vec![tag_id.clone()]),
        },
    )
    .await
    .unwrap();
    create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    let snap_dir = store::list_snapshots(&h.home.join(SNAPSHOTS_DIR)).unwrap()[0]
        .dir
        .clone();

    change_pw(&mut session, &h, "new-hunter2").await;
    lock_vault(session).await.unwrap();

    let new_kek = derive_kek(&h, "new-hunter2");
    let db = VaultDbConnection::open(&snap_dir.join("vault.vdb"))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let tags = repo.all_tags().await.unwrap();
    let tag = tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(
        read_dek_tag_name(&h, &new_kek, tag),
        "github",
        "the snapshot's tag re-wrapped and reads under the NEW KEK"
    );
    drop(repo);
    db.close().await.unwrap();
}

// 🔴 The B2 test (a legacy NULL snapshot tag MIGRATED, not skipped, on a password change) was
// REMOVED in slice 5.9 ②: its scenario is retired. A legacy snapshot tag no longer migrates — the
// snapshot rewrap now fails on it gracefully (non-fatal, badged stale, per rewrap_snapshots's L1),
// and the retirement guard `retired_legacy_tag_hard_fails_unlock` covers the "legacy → hard fail"
// property. The audit (`mise tag-legacy-audit`) proved no reachable vault carries such a tag.
