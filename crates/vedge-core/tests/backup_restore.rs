//! Backup snapshot + archive (slice 5.2a). The restore-side tests (verify-before-
//! touch, the journal state machine, refusals) land in 5.2b; here we prove the
//! archive is produced live and is well-formed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::needless_pass_by_value
)]

mod common;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::VaultRepository;
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    BackupVaultInput, CreateEntryInput, UnlockVaultInput, backup_vault, create_entry,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::backup::archive;
use vedge_core::infrastructure::backup::manifest::VAULT_MEMBER;
use vedge_core::infrastructure::sqlite::vault::{SqliteVaultRepository, VaultDbConnection};

async fn unlock(h: &Harness) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: h.vdb_path.clone(),
            master_password: h.master_password.clone(),
            secret_key: None,
        })
        .await
        .unwrap()
}

fn login(name: &str) -> EntryPayload {
    EntryPayload::Login(LoginPayload {
        meta: CommonMeta::new(name, EntryType::Login),
        username: "u".into(),
        password: secrecy::SecretString::from("p"),
        totp_secret: None,
        totp_params: vedge_core::TotpParams::default(),
        recovery_codes: vec![],
    })
}

async fn add(session: &mut VaultSession, name: &str) {
    create_entry(
        session,
        CreateEntryInput {
            payload: login(name),
        },
    )
    .await
    .unwrap();
}

/// Write a raw blob file straight into the vault's `.vedge_blobs/` dir (backup
/// copies the directory verbatim — the bytes' provenance doesn't matter to 5.2a).
fn seed_blob(h: &Harness, bytes: &[u8]) {
    let path = h
        .blob
        .root()
        .join(format!("{}.blob", EntryId::new().as_str()));
    std::fs::write(path, bytes).unwrap();
}

#[tokio::test]
async fn backup_produces_a_well_formed_archive_and_one_audit_row() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    add(&mut session, "c").await;
    seed_blob(&h, b"\x00\x01\x02 nonce-and-ciphertext");

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("vault-20260713-101500.vbk");
    let report = backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    // The report + on-disk artifacts.
    assert!(dest.exists(), "the .vbk archive should exist");
    assert_eq!(report.entry_count, 3);
    assert_eq!(report.blob_count, 1);
    assert!(report.archive_bytes > 0);

    // The advisory sidecar carries the whole-archive hash.
    let sidecar = out.path().join("vault-20260713-101500.vbk.blake3");
    let sidecar_hex = std::fs::read_to_string(&sidecar).unwrap();
    assert_eq!(sidecar_hex.trim(), report.archive_blake3);

    // The archive verifies: manifest present, every member's BLAKE3 matches.
    let manifest = archive::verify_archive(&dest).unwrap();
    assert_eq!(manifest.format_version, 1);
    assert_eq!(manifest.entry_count, 3);
    assert_eq!(manifest.blob_count, 1);
    // `files` covers vault.vdb + the one blob (manifest.json excludes itself).
    assert_eq!(manifest.files.len(), 2);
    assert!(manifest.files.iter().any(|m| m.name == VAULT_MEMBER));
    assert!(manifest.files.iter().any(|m| m.name.starts_with("blobs/")));

    // Exactly one BackupCreated audit row — never Exported.
    let audits = h.repo.recent_audit(200).await.unwrap();
    let created = audits
        .iter()
        .filter(|e| e.action == AuditAction::BackupCreated)
        .count();
    assert_eq!(created, 1, "exactly one BackupCreated row");
    assert!(
        !audits.iter().any(|e| e.action == AuditAction::Exported),
        "a backup must never write an Exported row"
    );
}

#[tokio::test]
async fn backup_snapshot_is_a_consistent_openable_db() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "one").await;
    add(&mut session, "two").await;
    add(&mut session, "three").await;

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("snap.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    // Extract the VACUUM INTO snapshot and open it as a standalone DB — it must
    // be internally consistent and carry exactly the same entries.
    let snap = out.path().join("extracted.vdb");
    archive::extract_member(&dest, VAULT_MEMBER, &snap).unwrap();
    let db = VaultDbConnection::open(&snap).await.unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(repo.all_entries().await.unwrap().len(), 3);
}

#[tokio::test]
async fn backup_without_documents_omits_blobs() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "solo").await;

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("noblobs.vbk");
    let report = backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    assert_eq!(report.blob_count, 0);
    let manifest = archive::verify_archive(&dest).unwrap();
    assert_eq!(manifest.blob_count, 0);
    // Only vault.vdb — no blob members.
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].name, VAULT_MEMBER);
}
