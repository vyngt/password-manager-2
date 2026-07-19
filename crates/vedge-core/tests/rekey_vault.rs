//! True re-key (slice 5.8): `rekey_vault` mints a FRESH DEK per entry and re-encrypts every
//! ciphertext surface — entries, EVERY history version, document blobs, and tags — under new DEKs
//! + a new KEK, crash-safely (stage-and-swap), retiring the old snapshot store.
//!
//! The signature test (spec ①) is `rekey_reencrypts_every_surface_under_the_new_dek`: it seeds an
//! entry with history + a document blob + a tag, re-keys, and asserts every surface decrypts under
//! ONLY the new DEK and FAILS under the old. It was watched to FAIL on an entries-only pass (with
//! the history / blob / tag re-encryption removed) before it passed — the guard that makes a
//! silently-partial re-key detectable, alongside the `..`-less `EntryPayload` match in the core.
//!
//! Like the in-session revert (which also swaps the home), each test drops the harness's OWN
//! repo/blob handles first, leaving only the live SESSION — whose DB the use case closes itself
//! before the swap (Windows will not rename a home holding an open `.vdb`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use common::{Harness, build_unlock};
use secrecy::SecretString;
use zeroize::Zeroizing;

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, BlobStore, CryptoProvider, KeyDerivationProvider, KeychainProvider,
    VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    CreateEntryInput, ImportDocumentInput, RekeyOutcome, RekeyVaultInput, UnlockVaultInput,
    UpdateEntryInput, create_entry, create_snapshot, import_document, lock_vault, rekey_vault,
    update_entry,
};
use vedge_core::domain::shared::{EntryId, SNAPSHOTS_DIR, TagId, VAULT_FILE, now};
use vedge_core::domain::vault::aad::{entry_aad, tag_aad};
use vedge_core::domain::vault::crypto_constants::{DEK_LEN, KEK_LEN, SECRET_KEY_LEN};
use vedge_core::domain::vault::entities::{EntryHistoryRow, EntryRow, TagRow, VaultConfig};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{
    CommonMeta, EntryPayload, EntryType, LoginPayload, TagPayload,
};
use vedge_core::infrastructure::blob::FilesystemBlobStore;
use vedge_core::infrastructure::snapshot::manifest::SnapshotReason;
use vedge_core::infrastructure::snapshot::store;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

const OLD_PW: &str = "correct horse battery staple";

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

/// A live vault ready to re-key, with the harness's own `vault.vdb` handle already dropped so
/// only the session holds it. Carries the ports + the material tests need to re-derive KEKs and
/// re-open the re-keyed vault, plus the seeded ids and the captured OLD DEKs.
struct Live {
    _tempdir: tempfile::TempDir,
    session: VaultSession,
    home: PathBuf,
    crypto: Arc<dyn CryptoProvider>,
    kdf: Arc<dyn KeyDerivationProvider>,
    keychain: Arc<dyn KeychainProvider>,
    biometric: Arc<dyn BiometricAuthenticator>,
    secret_key: [u8; SECRET_KEY_LEN],
    old_kek: [u8; KEK_LEN],
    config: VaultConfig,
    vault_uuid: String,
    login_id: EntryId,
    doc_id: EntryId,
    tag_id: TagId,
    doc_bytes: Vec<u8>,
    old_login_dek: [u8; DEK_LEN],
    old_doc_dek: [u8; DEK_LEN],
    /// `schema_version` AFTER unlock (the 5.6.0 tag-key migration bumps 1 → 2) — re-key must
    /// preserve it, not bump it (spec ⑦).
    schema_before: i32,
}

/// Seed a tag, a login (edited twice → 3 versions), and a document, then hand back a live session
/// with the harness's own handles dropped.
async fn build_live() -> Live {
    let h = Harness::fresh().await;
    // A DEK-sealed tag, sealed under the ORIGINAL KEK (re-key must re-seal it under the new one).
    let tag_id = h.seed_tag("work").await;

    let crypto: Arc<dyn CryptoProvider> = Arc::clone(&h.crypto) as _;
    let kdf: Arc<dyn KeyDerivationProvider> = Arc::clone(&h.kdf) as _;
    let keychain: Arc<dyn KeychainProvider> = Arc::clone(&h.keychain) as _;
    let biometric: Arc<dyn BiometricAuthenticator> = Arc::clone(&h.biometric) as _;

    let unlock = build_unlock(&h);
    let mut session = unlock
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap();

    // A login with real version history: v1 → v2 → v3.
    let login_id = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login("github", "pw-v1"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    update_entry(
        &mut session,
        UpdateEntryInput::full(login_id.clone(), login("github", "pw-v2")),
    )
    .await
    .unwrap();
    update_entry(
        &mut session,
        UpdateEntryInput::full(login_id.clone(), login("github", "pw-v3")),
    )
    .await
    .unwrap();

    // A document with a real external blob.
    let doc_bytes: Vec<u8> = (0..5_000u32).flat_map(u32::to_le_bytes).collect();
    let doc_id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "passport.pdf".into(),
            mime_type: "application/pdf".into(),
            content: doc_bytes.clone(),
            meta: CommonMeta::new("passport", EntryType::Document),
        },
    )
    .await
    .unwrap();

    // Capture the OLD DEKs (what a copied-file attacker would hold) BEFORE dropping the handles.
    let login_row = h.repo.get_entry(&login_id).await.unwrap();
    let old_login_dek = *h.crypto.unwrap_dek(&login_row.dek_wrapped, &h.kek).unwrap();
    let doc_row = h.repo.get_entry(&doc_id).await.unwrap();
    let old_doc_dek = *h.crypto.unwrap_dek(&doc_row.dek_wrapped, &h.kek).unwrap();
    // The CURRENT (post-unlock) schema version — unlock already ran the 5.6.0 tag migration.
    let schema_before = h.repo.load_config().await.unwrap().schema_version;

    let Harness {
        tempdir,
        home,
        secret_key,
        config,
        vault_uuid,
        kek,
        repo,
        blob,
        ..
    } = h;
    // Only the live session may hold `vault.vdb` when the swap runs.
    drop(repo);
    drop(blob);

    Live {
        _tempdir: tempdir,
        session,
        home,
        crypto,
        kdf,
        keychain,
        biometric,
        secret_key,
        old_kek: kek,
        config,
        vault_uuid,
        login_id,
        doc_id,
        tag_id,
        doc_bytes,
        old_login_dek,
        old_doc_dek,
        schema_before,
    }
}

/// Derive the `(kek, verify_hash)` a password + the vault's Secret Key would produce.
fn derive(
    kdf: &dyn KeyDerivationProvider,
    config: &VaultConfig,
    sk: &[u8; SECRET_KEY_LEN],
    pw: &str,
) -> ([u8; KEK_LEN], [u8; 32]) {
    let input = kdf.preprocess_2skd(pw.as_bytes(), sk).unwrap();
    let mk = kdf
        .derive_master_key(&input, &config.vault_salt, &config.kdf_params)
        .unwrap();
    let verify = kdf.derive_verify_hash(&mk).unwrap();
    let kek = kdf.derive_kek(&mk).unwrap();
    (*kek, verify)
}

async fn run_rekey(
    live: &Live,
    new_pw: &str,
    new_sk: Option<[u8; SECRET_KEY_LEN]>,
    cancel: &AtomicBool,
) -> RekeyOutcome {
    let noop = |_done: u64, _total: u64| {};
    rekey_vault(
        &live.session,
        Arc::clone(&live.kdf),
        Arc::clone(&live.keychain),
        Arc::clone(&live.biometric),
        RekeyVaultInput {
            new_password: Zeroizing::new(new_pw.to_owned()),
            new_secret_key: new_sk.map(Zeroizing::new),
        },
        &noop,
        cancel,
    )
    .await
    .unwrap()
}

/// Read every entry + tag + config from a vault home, closing the connection cleanly.
async fn dump(home: &Path) -> (Vec<EntryRow>, Vec<TagRow>, VaultConfig) {
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let entries = repo.all_entries().await.unwrap();
    let tags = repo.all_tags().await.unwrap();
    let config = repo.load_config().await.unwrap();
    drop(repo);
    db.close().await.unwrap();
    (entries, tags, config)
}

async fn history_of(home: &Path, entry_id: &EntryId) -> Vec<EntryHistoryRow> {
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let h = repo.list_history(entry_id).await.unwrap();
    drop(repo);
    db.close().await.unwrap();
    h
}

fn row<'a>(rows: &'a [EntryRow], id: &EntryId) -> &'a EntryRow {
    rows.iter().find(|r| &r.id == id).expect("entry present")
}

/// 🔴 Spec ① — THE completeness guard. After a re-key, EVERY ciphertext surface (entry, every
/// history version, the document blob, and the tag) decrypts under ONLY the new DEK and FAILS
/// under the old. Watched to fail on an entries-only pass first.
#[tokio::test]
async fn rekey_reencrypts_every_surface_under_the_new_dek() {
    let live = build_live().await;
    let cancel = AtomicBool::new(false);

    let outcome = run_rekey(&live, "brand-new-password", None, &cancel).await;
    assert!(matches!(outcome, RekeyOutcome::Rekeyed { .. }));
    // The session's DB was closed by the re-key; keep the (inert) object so `live` stays whole.

    let (new_kek, new_verify) = derive(
        live.kdf.as_ref(),
        &live.config,
        &live.secret_key,
        "brand-new-password",
    );
    let (entries, tags, config) = dump(&live.home).await;

    // --- the live entry ciphertexts open under the NEW KEK, and NOT the old ---
    let login_row = row(&entries, &live.login_id);
    let new_login_dek = live
        .crypto
        .unwrap_dek(&login_row.dek_wrapped, &new_kek)
        .unwrap();
    let aad = entry_aad(&live.login_id, login_row.version).unwrap();
    live.crypto
        .decrypt_entry(
            &new_login_dek,
            &login_row.nonce,
            &login_row.ciphertext,
            &aad,
        )
        .expect("live entry must decrypt under the new DEK");
    assert!(
        live.crypto
            .unwrap_dek(&login_row.dek_wrapped, &live.old_kek)
            .is_err(),
        "the wrapped DEK must NOT unwrap under the old KEK"
    );

    // --- EVERY history version (the landmine) opens under the NEW DEK ---
    let versions = history_of(&live.home, &live.login_id).await;
    assert_eq!(versions.len(), 2, "v1 + v2 preserved as history");
    for v in &versions {
        let v_aad = entry_aad(&live.login_id, v.version).unwrap();
        live.crypto
            .decrypt_entry(&new_login_dek, &v.nonce, &v.ciphertext, &v_aad)
            .expect("history version must decrypt under the new DEK");
        assert!(
            live.crypto
                .decrypt_entry(&live.old_login_dek, &v.nonce, &v.ciphertext, &v_aad)
                .is_err(),
            "history version must NOT decrypt under the old DEK"
        );
    }

    // --- the document blob opens under the NEW DEK with the re-stamped nonce ---
    let doc_row = row(&entries, &live.doc_id);
    let new_doc_dek = live
        .crypto
        .unwrap_dek(&doc_row.dek_wrapped, &new_kek)
        .unwrap();
    let doc_aad = entry_aad(&live.doc_id, doc_row.version).unwrap();
    let doc_plain = live
        .crypto
        .decrypt_entry(&new_doc_dek, &doc_row.nonce, &doc_row.ciphertext, &doc_aad)
        .unwrap();
    let EntryPayload::Document(doc) = EntryPayload::from_decrypted_json(&doc_plain).unwrap() else {
        panic!("expected a Document payload");
    };
    let blob_store = FilesystemBlobStore::new(&live.home, Arc::clone(&live.crypto)).unwrap();
    let read = blob_store
        .read_blob(&live.doc_id, &new_doc_dek, &doc.blob_nonce)
        .await
        .expect("blob must decrypt under the new DEK + re-stamped nonce");
    assert_eq!(
        &*read, &live.doc_bytes,
        "blob bytes round-trip through re-key"
    );

    // --- the tag opens under the NEW KEK, and NOT the old ---
    let tag = tags
        .iter()
        .find(|t| t.id == live.tag_id)
        .expect("tag present");
    let tag_dek = live
        .crypto
        .unwrap_dek(&tag.dek_wrapped.unwrap(), &new_kek)
        .expect("tag DEK must unwrap under the new KEK");
    let t_aad = tag_aad(&live.tag_id).unwrap();
    let tag_plain = live
        .crypto
        .decrypt_entry(&tag_dek, &tag.nonce, &tag.ciphertext, &t_aad)
        .unwrap();
    let tag_payload: TagPayload = serde_json::from_slice(&tag_plain).unwrap();
    assert_eq!(tag_payload.name, "work");
    assert!(
        live.crypto
            .unwrap_dek(&tag.dek_wrapped.unwrap(), &live.old_kek)
            .is_err(),
        "tag DEK must NOT unwrap under the old KEK"
    );

    // --- config: new verify_hash, recovery slot nulled, schema unchanged ---
    assert_eq!(config.verify_hash, new_verify);
    assert!(config.recovery_slot.is_none());
    assert_eq!(
        config.schema_version, live.schema_before,
        "re-key must not bump schema_version"
    );
}

/// 🔴 The distinguishing property (spec test 2): a captured OLD DEK opens NOTHING that exists
/// after the re-key — the current entry ciphertext and the current blob both refuse it. This is
/// what re-key does that a rewrap cannot.
#[tokio::test]
async fn captured_old_deks_open_nothing_after_rekey() {
    let live = build_live().await;
    let cancel = AtomicBool::new(false);
    let outcome = run_rekey(&live, "new-pw-2", None, &cancel).await;
    assert!(matches!(outcome, RekeyOutcome::Rekeyed { .. }));

    let (entries, _tags, _config) = dump(&live.home).await;

    // The current login ciphertext refuses the captured old DEK.
    let login_row = row(&entries, &live.login_id);
    let aad = entry_aad(&live.login_id, login_row.version).unwrap();
    assert!(
        live.crypto
            .decrypt_entry(
                &live.old_login_dek,
                &login_row.nonce,
                &login_row.ciphertext,
                &aad
            )
            .is_err(),
        "the current entry must not open under a captured old DEK"
    );

    // The current blob on disk refuses the captured old document DEK. Read the re-stamped nonce
    // from the re-keyed payload just to feed the blob API; the DEK mismatch is what fails it.
    let doc_row = row(&entries, &live.doc_id);
    let (new_kek, _v) = derive(
        live.kdf.as_ref(),
        &live.config,
        &live.secret_key,
        "new-pw-2",
    );
    let new_doc_dek = live
        .crypto
        .unwrap_dek(&doc_row.dek_wrapped, &new_kek)
        .unwrap();
    let doc_aad = entry_aad(&live.doc_id, doc_row.version).unwrap();
    let doc_plain = live
        .crypto
        .decrypt_entry(&new_doc_dek, &doc_row.nonce, &doc_row.ciphertext, &doc_aad)
        .unwrap();
    let EntryPayload::Document(doc) = EntryPayload::from_decrypted_json(&doc_plain).unwrap() else {
        panic!("expected a Document payload");
    };
    let blob_store = FilesystemBlobStore::new(&live.home, Arc::clone(&live.crypto)).unwrap();
    assert!(
        blob_store
            .read_blob(&live.doc_id, &live.old_doc_dek, &doc.blob_nonce)
            .await
            .is_err(),
        "the current blob must not open under a captured old DEK"
    );
}

/// 🔴 Spec ③ — a re-key RETIRES the snapshot store (a pre-re-key snapshot's bodies are on the OLD
/// DEKs; reverting to it would restore the state re-key burned).
#[tokio::test]
async fn rekey_retires_the_snapshot_store() {
    let live = build_live().await;
    // A snapshot taken BEFORE the re-key.
    create_snapshot(&live.session, SnapshotReason::Manual)
        .await
        .unwrap();
    let store_dir = live.home.join(SNAPSHOTS_DIR);
    assert_eq!(store::list_snapshots(&store_dir).unwrap().len(), 1);

    let cancel = AtomicBool::new(false);
    let outcome = run_rekey(&live, "new-pw-3", None, &cancel).await;
    assert!(matches!(outcome, RekeyOutcome::Rekeyed { .. }));

    assert!(
        store::list_snapshots(&store_dir).unwrap().is_empty(),
        "the pre-re-key snapshot store must be retired, not carried across the swap"
    );
}

/// 🔴 Spec ④ — superset semantics: re-key with a new password AND a rotated Secret Key opens on
/// the new credentials, refuses the old, nulls the recovery slot, and re-stores the new SK.
#[tokio::test]
async fn rekey_opens_on_new_credentials_and_nulls_recovery() {
    let live = build_live().await;
    // Arm a recovery slot so we can prove the re-key nulls it (5.7 ③, inherited).
    {
        let db = VaultDbConnection::open(&live.home.join(VAULT_FILE))
            .await
            .unwrap();
        let repo = SqliteVaultRepository::new(db.handle());
        repo.set_recovery_slot(&[0x11u8; 40]).await.unwrap();
        // Stamp a "last snapshot" so we can prove the re-key retire nulls it (5.9 ④).
        repo.touch_last_snapshot_at(now()).await.unwrap();
        drop(repo);
        db.close().await.unwrap();
    }

    let new_sk = [0xEEu8; SECRET_KEY_LEN];
    let cancel = AtomicBool::new(false);
    let outcome = run_rekey(&live, "new-pw-4", Some(new_sk), &cancel).await;
    assert!(matches!(outcome, RekeyOutcome::Rekeyed { .. }));

    // The keychain got the rotated SK.
    assert_eq!(
        *live.keychain.read_secret_key(&live.vault_uuid).unwrap(),
        new_sk
    );

    // Re-unlock with the new password succeeds and sees every entry; the old password fails.
    let uv = build_unlock_for(&live);
    let session_new = uv
        .execute(UnlockVaultInput {
            vault_path: live.home.clone(),
            master_password: Zeroizing::new("new-pw-4".into()),
            secret_key: None,
        })
        .await
        .unwrap();
    assert_eq!(session_new.index().all_active().len(), 2); // login + document
    lock_vault(session_new).await.unwrap();

    let err = build_unlock_for(&live)
        .execute(UnlockVaultInput {
            vault_path: live.home.clone(),
            master_password: Zeroizing::new(OLD_PW.into()),
            secret_key: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::WrongCredentials));

    let (_e, _t, config) = dump(&live.home).await;
    assert!(
        config.recovery_slot.is_none(),
        "a re-key nulls the recovery slot"
    );
    // Credential-age stamps (5.9 ③): re-key with a new SK refreshes both.
    assert!(
        config.last_password_change_at.is_some(),
        "a re-key stamps the password change"
    );
    assert!(
        config.last_secret_key_rotation_at.is_some(),
        "a re-key with a new Secret Key stamps the rotation"
    );
    // Re-key RETIRES the snapshot store, so the stamp must not keep claiming one (5.9 ④).
    assert!(
        config.last_snapshot_at.is_none(),
        "a re-key retires snapshots and nulls last_snapshot_at"
    );
}

/// A cancel BEFORE the commit point discards staging and leaves the live vault untouched — the
/// session stays alive and the OLD password still opens it.
#[tokio::test]
async fn rekey_cancel_before_commit_leaves_the_vault_untouched() {
    let live = build_live().await;
    let cancel = AtomicBool::new(true); // cancel is seen at the first entry, before any swap.

    let outcome = run_rekey(&live, "never-applied", None, &cancel).await;
    assert!(matches!(outcome, RekeyOutcome::Cancelled));

    // The OLD password still opens the untouched vault (the session's DB was never closed).
    let session = build_unlock_for(&live)
        .execute(UnlockVaultInput {
            vault_path: live.home.clone(),
            master_password: Zeroizing::new(OLD_PW.into()),
            secret_key: None,
        })
        .await
        .unwrap();
    assert_eq!(session.index().all_active().len(), 2);
    lock_vault(session).await.unwrap();
}

/// Build an `UnlockVault` from a `Live`'s ports (the harness is already consumed).
fn build_unlock_for(live: &Live) -> vedge_core::application::vault::use_cases::UnlockVault {
    use vedge_core::application::vault::ports::{
        BlobStoreFactory, ClipboardProvider, VaultRepositoryFactory,
    };
    use vedge_core::application::vault::use_cases::UnlockVault;
    use vedge_core::infrastructure::blob::FilesystemBlobStoreFactory;
    use vedge_core::infrastructure::clipboard::MemoryClipboardProvider;
    use vedge_core::infrastructure::sqlite::vault::SqliteVaultRepositoryFactory;

    UnlockVault {
        repo_factory: Arc::new(SqliteVaultRepositoryFactory::new())
            as Arc<dyn VaultRepositoryFactory>,
        blob_factory: Arc::new(FilesystemBlobStoreFactory::new()) as Arc<dyn BlobStoreFactory>,
        crypto: Arc::clone(&live.crypto),
        clipboard: Arc::new(MemoryClipboardProvider::new()) as Arc<dyn ClipboardProvider>,
        kdf: Arc::clone(&live.kdf),
        keychain: Arc::clone(&live.keychain),
    }
}
