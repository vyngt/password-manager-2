//! The tweezers (slice 5.3c): recover individual entries FROM a local snapshot into
//! the live vault, without reverting the whole vault.
//!
//! Proves spec #14 (a deleted entry comes back under a fresh DEK/ULID, the rest of the
//! vault untouched), spec #15 (a stale snapshot is refused, never silently skipped),
//! and Decision 2A (a document recovers byte-exact through the object pool).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use common::Harness;
use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::unlock_vault::UnlockVaultInput;
use vedge_core::domain::export::{ExportBundle, ExportEntry, ExportLogin};
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::export::{archive, envelope};
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};
use vedge_core::{
    CreateEntryInput, ExportEntriesInput, ExportFormat, ImportAction, ImportDocumentInput,
    RowAction, SnapshotReason, begin_snapshot_import, commit_import, create_entry, create_snapshot,
    create_tag, hard_delete_entry, import_document,
};

const PASSPHRASE: &str = "a strong tweezers passphrase";

async fn unlock(h: &Harness) -> VaultSession {
    common::build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.home.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login_payload(name: &str, username: &str, password: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: username.to_owned(),
        password: SecretString::from(password.to_owned()),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn export_to_bytes(session: &VaultSession) -> Vec<u8> {
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("x.vedgex");
    let _ = vedge_core::export_entries(
        session,
        ExportEntriesInput {
            dest: dest.clone(),
            format: ExportFormat::Encrypted {
                passphrase: Zeroizing::new(PASSPHRASE.to_owned()),
            },
        },
    )
    .await
    .unwrap();
    std::fs::read(&dest).unwrap()
}

fn open_tar(sealed: &[u8]) -> Vec<u8> {
    envelope::open(PASSPHRASE.as_bytes(), sealed)
        .unwrap()
        .to_vec()
}

fn bundle_of(tar: &[u8]) -> ExportBundle {
    let entries_json = archive::read_member(tar, archive::ENTRIES_MEMBER)
        .unwrap()
        .unwrap();
    serde_json::from_slice(&entries_json).unwrap()
}

fn find_login<'a>(bundle: &'a ExportBundle, name: &str) -> &'a ExportLogin {
    bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Login(l) if l.meta.name == name => Some(l),
            _ => None,
        })
        .unwrap_or_else(|| panic!("login {name} missing"))
}

fn count_login(bundle: &ExportBundle, name: &str) -> usize {
    bundle
        .entries
        .iter()
        .filter(|e| matches!(e, ExportEntry::Login(l) if l.meta.name == name))
        .count()
}

/// 🟢 spec #14: delete an entry, then recover exactly it from a snapshot — it lives
/// again under a FRESH ULID (the source's is gone), its secret intact, and the rest of
/// the vault is untouched (no double-import of the sibling that was never deleted).
#[tokio::test]
async fn tweezers_recovers_a_deleted_entry_from_a_snapshot() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let keeper = create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("keeper", "me", "=s3cret"),
        },
    )
    .await
    .unwrap()
    .entry_id;
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("other", "you", "p2"),
        },
    )
    .await
    .unwrap();

    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    // The one entry is gone from the live vault.
    hard_delete_entry(&mut session, &keeper).await.unwrap();
    assert_eq!(session.index().all_active().len(), 1, "keeper is gone");

    // Recover ONLY keeper from the snapshot (the preview shows both).
    let rows = begin_snapshot_import(&mut session, &snap.id).await.unwrap();
    assert_eq!(rows.len(), 2, "snapshot holds both entries");
    let keeper_row = rows.iter().find(|r| r.name == "keeper").unwrap().row_id;
    // ⑦ the preview shows presence, never the secret.
    assert!(
        rows.iter()
            .find(|r| r.name == "keeper")
            .unwrap()
            .has_password
    );
    let report = commit_import(
        &mut session,
        &[ImportAction {
            row_id: keeper_row,
            action: RowAction::Import,
        }],
    )
    .await
    .unwrap();
    assert_eq!(report.imported, 1);
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    let bundle = bundle_of(&open_tar(&export_to_bytes(&session).await));
    // keeper is back, secret intact, under a FRESH ULID (not the deleted one).
    let recovered = find_login(&bundle, "keeper");
    assert_eq!(recovered.password.expose_secret(), "=s3cret");
    assert_ne!(
        recovered.meta.id,
        keeper.as_str(),
        "#13 — a fresh ULID, not the snapshot's"
    );
    // The sibling was never deleted and never double-imported → exactly one of each.
    assert_eq!(count_login(&bundle, "keeper"), 1);
    assert_eq!(count_login(&bundle, "other"), 1);
}

/// spec #15: a snapshot whose `verify_hash_prefix` no longer matches the live vault (a
/// failed ⑬ rewrap) is REFUSED with `SnapshotStale` — never opened silently.
#[tokio::test]
async fn a_stale_snapshot_is_refused_not_silently_skipped() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    create_entry(
        &mut session,
        CreateEntryInput {
            payload: login_payload("x", "u", "p"),
        },
    )
    .await
    .unwrap();
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();

    // Forge staleness: rewrite the manifest's verify_hash_prefix so it no longer
    // matches the live vault (as a failed rewrap would leave it).
    let manifest_path = h
        .home
        .join("snapshots")
        .join(&snap.id)
        .join("manifest.json");
    let mut m: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    m["verify_hash_prefix"] = serde_json::Value::String("deadbeefdeadbeef".to_owned());
    std::fs::write(&manifest_path, serde_json::to_vec(&m).unwrap()).unwrap();

    let err = begin_snapshot_import(&mut session, &snap.id)
        .await
        .unwrap_err();
    assert!(matches!(err, VaultError::SnapshotStale), "got {err:?}");
}

/// Decision 2A: a document recovers from a snapshot BYTE-EXACT — its blob is read from
/// the content-addressed object pool, decrypted under the snapshot entry's DEK, and
/// re-encrypted under a fresh DEK on commit.
#[tokio::test]
async fn a_document_recovers_from_a_snapshot_byte_exact() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let content = b"PDF-ish bytes \x00\x01\x02 the confidential report".to_vec();
    let doc_id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "report.pdf".to_owned(),
            mime_type: "application/pdf".to_owned(),
            content: content.clone(),
            meta: CommonMeta::new("report", EntryType::Document),
        },
    )
    .await
    .unwrap();
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    hard_delete_entry(&mut session, &doc_id).await.unwrap();

    let rows = begin_snapshot_import(&mut session, &snap.id).await.unwrap();
    let doc_row = rows
        .iter()
        .find(|r| r.entry_type == "document")
        .unwrap()
        .row_id;
    let report = commit_import(
        &mut session,
        &[ImportAction {
            row_id: doc_row,
            action: RowAction::Import,
        }],
    )
    .await
    .unwrap();
    assert_eq!(report.imported, 1);
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    // Re-export and read the recovered document's plaintext blob from the tar.
    let tar = open_tar(&export_to_bytes(&session).await);
    let bundle = bundle_of(&tar);
    let doc = bundle
        .entries
        .iter()
        .find_map(|e| match e {
            ExportEntry::Document(d) => Some(d),
            _ => None,
        })
        .expect("a document entry");
    assert_ne!(
        doc.meta.id,
        doc_id.as_str(),
        "a fresh ULID, not the snapshot's"
    );
    let member = format!("{}{}", archive::BLOBS_PREFIX, doc.meta.id);
    let blob = archive::read_member(&tar, &member).unwrap().unwrap();
    assert_eq!(
        blob.as_slice(),
        content.as_slice(),
        "the document bytes survived the snapshot round-trip"
    );
}

/// Break the AEAD auth tag on every tag row in a snapshot's `vault.vdb`, so the tag table no
/// longer decrypts under the current KEK — the state a credential change leaves (the ⑬ rewrap
/// re-wraps entry DEKs + `verify_hash` but NOT tag rows). Entries + `verify_hash` are untouched.
async fn corrupt_snapshot_tags(h: &Harness, snap_id: &str) {
    let vdb = h.home.join("snapshots").join(snap_id).join("vault.vdb");
    let db = VaultDbConnection::open(&vdb).await.unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let mut tags = repo.all_tags().await.unwrap();
    assert!(!tags.is_empty(), "the snapshot must carry a tag to corrupt");
    for row in &mut tags {
        row.ciphertext[0] ^= 0xFF; // flip a byte → Poly1305 auth fails → decrypt errors
        repo.update_tag(row).await.unwrap();
    }
    drop(repo);
    db.close().await.ok();
}

/// 🔴 Regression (code-review): a snapshot's tag rows can be unreadable under the current KEK
/// (a credential change re-wraps entry DEKs + `verify_hash` but NOT tags). Recovering a TAGGED
/// entry from such a snapshot must still SUCCEED — tags degrade to none — and must NOT be
/// refused as `SnapshotStale` (the entries decrypt fine).
#[tokio::test]
async fn tweezers_recovers_when_snapshot_tags_are_unreadable() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;

    let tag = create_tag(&mut session, "work", None).await.unwrap();
    let mut payload = login_payload("keeper", "me", "=s3cret");
    if let EntryPayload::Login(l) = &mut payload {
        l.meta.tag_ids = vec![tag];
    }
    let keeper = create_entry(&mut session, CreateEntryInput { payload })
        .await
        .unwrap()
        .entry_id;
    let snap = create_snapshot(&session, SnapshotReason::Manual)
        .await
        .unwrap();
    // Delete the original so the ONLY "keeper" afterward is the recovered (untagged) copy.
    hard_delete_entry(&mut session, &keeper).await.unwrap();

    corrupt_snapshot_tags(&h, &snap.id).await;

    // Recovery must proceed (not abort as stale) — the entry decrypts, only the tags don't.
    let rows = begin_snapshot_import(&mut session, &snap.id).await.unwrap();
    assert_eq!(rows.len(), 1, "the tagged entry is still recoverable");
    let row_id = rows[0].row_id;
    let report = commit_import(
        &mut session,
        &[ImportAction {
            row_id,
            action: RowAction::Import,
        }],
    )
    .await
    .unwrap();
    assert_eq!(report.imported, 1);
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    let bundle = bundle_of(&open_tar(&export_to_bytes(&session).await));
    let recovered = find_login(&bundle, "keeper");
    assert_eq!(recovered.password.expose_secret(), "=s3cret");
    assert!(
        recovered.meta.tags.is_empty(),
        "unreadable snapshot tags degrade to none, not a failed recovery"
    );
}
