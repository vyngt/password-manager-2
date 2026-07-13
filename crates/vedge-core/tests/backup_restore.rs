//! Backup + restore host suite (slices 5.2a / 5.2b).
//!
//! 5.2a proves the archive is produced live and well-formed; 5.2b proves the
//! journaled restore round-trips a vault (entries + a document blob) and refuses a
//! tampered archive, a uuid mismatch, a newer schema, and an unknown format. The
//! crash-at-every-state matrix is a unit test in `infrastructure::backup::journal`
//! (it drives the state machine directly, more precisely than a use-case fault
//! point could).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    clippy::needless_pass_by_value
)]

mod common;

use std::path::Path;

use zeroize::Zeroizing;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::{KeychainProvider, VaultRepository};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    BackupVaultInput, CreateEntryInput, ImportDocumentInput, InspectBackupInput, RestoreVaultInput,
    UnlockVaultInput, backup_vault, create_entry, export_document, import_document, inspect_backup,
    restore_vault,
};
use vedge_core::domain::shared::EntryId;
use vedge_core::domain::vault::entities::AuditAction;
use vedge_core::domain::vault::errors::VaultError;
use vedge_core::domain::vault::payloads::{CommonMeta, EntryPayload, EntryType, LoginPayload};
use vedge_core::infrastructure::backup::archive;
use vedge_core::infrastructure::backup::manifest::{
    BACKUP_FORMAT_VERSION, BackupManifest, ManifestFile, VAULT_MEMBER,
};
use vedge_core::infrastructure::keychain::MemoryKeychainProvider;
use vedge_core::infrastructure::sqlite::vault::{
    SqliteVaultRepository, SqliteVaultRepositoryFactory, VaultDbConnection,
};

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

// ---- 5.2b: restore round-trip + refusals ----------------------------------

/// Unlock a vault at an arbitrary path with the harness's master password + an
/// explicit Secret Key (bypassing the keychain — the restored vault lives at a
/// path the keychain never saw).
async fn unlock_at(h: &Harness, path: &Path) -> VaultSession {
    build_unlock(h)
        .execute(UnlockVaultInput {
            vault_path: path.to_path_buf(),
            master_password: h.master_password.clone(),
            secret_key: Some(Zeroizing::new(h.secret_key)),
        })
        .await
        .unwrap()
}

/// Stamp a specific `vault_uuid` onto a harness vault (default harness uuid is
/// `None`, which can't exercise the mismatch path).
async fn set_uuid(h: &Harness, uuid: &str) {
    let mut cfg = h.config.clone();
    cfg.vault_uuid = Some(uuid.to_owned());
    h.repo.save_config(&cfg).await.unwrap();
}

/// Build a minimal, internally-consistent `.vbk` with a chosen format/schema — for
/// the refusal tests, whose gate fires *after* `verify_archive` passes.
fn write_min_archive(dir: &Path, format_version: u32, schema_version: i32) -> std::path::PathBuf {
    let vault = dir.join("vault.vdb");
    std::fs::write(&vault, b"dummy snapshot bytes -- never opened, only hashed").unwrap();
    let (size, blake3) = archive::hash_file(&vault).unwrap();
    let manifest = BackupManifest {
        format_version,
        vault_uuid: None,
        schema_version,
        created_at: "2026-07-13T00:00:00.000Z".to_owned(),
        entry_count: 0,
        blob_count: 0,
        commit_counter: 0,
        files: vec![ManifestFile {
            name: VAULT_MEMBER.to_owned(),
            size,
            blake3,
        }],
    };
    let manifest_path = dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let dest = dir.join("min.vbk");
    archive::write_archive(
        &[
            ("manifest.json".to_owned(), manifest_path),
            (VAULT_MEMBER.to_owned(), vault),
        ],
        &dest,
    )
    .unwrap();
    dest
}

/// The whole-slice proof: back up a live vault (3 logins + a document), restore it
/// into a fresh location, unlock it, and decrypt the document.
#[tokio::test]
async fn restore_round_trip_recovers_entries_and_a_document() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    add(&mut session, "c").await;
    let doc_id = import_document(
        &mut session,
        ImportDocumentInput {
            filename: "notes.txt".into(),
            mime_type: "text/plain".into(),
            content: b"the quick brown fox".to_vec(),
            meta: CommonMeta::new("notes", EntryType::Document),
        },
    )
    .await
    .unwrap();

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("full.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // Install-from-backup into a fresh, absent target (no open handles).
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("restored.vdb");
    let factory = SqliteVaultRepositoryFactory::new();
    let report = restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: target.clone(),
            archive_path: dest,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.entry_count, 4);
    assert_eq!(report.blob_count, 1);

    // Unlock the restored vault and decrypt the document — every layer round-trips.
    let restored = unlock_at(&h, &target).await;
    let (name, bytes) = export_document(&restored, &doc_id).await.unwrap();
    assert_eq!(name, "notes.txt");
    assert_eq!(bytes.to_vec(), b"the quick brown fox".to_vec());
    drop(restored);

    // Independent DB check: 4 entries + exactly one BackupRestored row.
    let db = VaultDbConnection::open(&target).await.unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(repo.all_entries().await.unwrap().len(), 4);
    let restored_rows = repo
        .recent_audit(200)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.action == AuditAction::BackupRestored)
        .count();
    assert_eq!(restored_rows, 1, "exactly one BackupRestored row");
}

/// Restore OVER an existing (locked, matching-uuid) vault — exercises the
/// move-aside + `.old` cleanup and, on Windows, the close-the-read-handle-before-
/// rename path that the fresh-install case never touches.
#[tokio::test]
async fn restore_over_an_existing_vault_replaces_it() {
    let h = Harness::fresh().await;
    set_uuid(&h, "01XXXXXXXXXXXXXXXXXXXXXXXX").await;
    let mut session = unlock(&h).await;
    add(&mut session, "keep-me").await;
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("over.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // An existing target with the same uuid (a copy of the source's `.vdb`).
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("existing.vdb");
    std::fs::copy(&h.vdb_path, &target).unwrap();

    let factory = SqliteVaultRepositoryFactory::new();
    restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: target.clone(),
            archive_path: dest,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap();

    // No transient artifacts survive.
    assert!(!target_dir.path().join("existing.vdb.old").exists());
    assert!(!target_dir.path().join(".existing.restore-staging").exists());
    assert!(!target_dir.path().join("existing.restore.intent").exists());

    // The restored vault opens and carries the backed-up entry.
    let restored = unlock_at(&h, &target).await;
    drop(restored);
    let db = VaultDbConnection::open(&target).await.unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    assert_eq!(repo.all_entries().await.unwrap().len(), 1);
}

#[tokio::test]
async fn restore_refuses_a_tampered_archive_and_leaves_the_target_untouched() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "x").await;
    seed_blob(&h, b"some blob bytes to enlarge the archive payload region");

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("tam.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // Flip a 64-byte run through the middle — lands in `vault.vdb`'s content (the
    // dominant member), so `verify_archive`'s per-file hash check fails.
    let mut bytes = std::fs::read(&dest).unwrap();
    let start = bytes.len() / 2;
    let end = (start + 64).min(bytes.len());
    for b in &mut bytes[start..end] {
        *b ^= 0xff;
    }
    std::fs::write(&dest, &bytes).unwrap();

    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("t.vdb");
    let factory = SqliteVaultRepositoryFactory::new();
    let err = restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: target,
            archive_path: dest,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::Storage(_)),
        "tamper → a Storage(Io) integrity refusal, got {err:?}"
    );
    // Refusal is at verify (before staging): the target dir stays empty.
    assert_eq!(
        std::fs::read_dir(target_dir.path()).unwrap().count(),
        0,
        "a refused restore must not create or stage anything"
    );
}

#[tokio::test]
async fn restore_refuses_a_uuid_mismatch() {
    let ha = Harness::fresh().await;
    set_uuid(&ha, "01AAAAAAAAAAAAAAAAAAAAAAAA").await;
    let mut sa = unlock(&ha).await;
    add(&mut sa, "one").await;
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("a.vbk");
    backup_vault(
        &sa,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    // A readable target vault with a DIFFERENT uuid.
    let hb = Harness::fresh().await;
    set_uuid(&hb, "01BBBBBBBBBBBBBBBBBBBBBBBB").await;

    let factory = SqliteVaultRepositoryFactory::new();
    let err = restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: hb.vdb_path.clone(),
            archive_path: dest,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::MalformedPayload(_)),
        "uuid mismatch → MalformedPayload, got {err:?}"
    );
}

#[tokio::test]
async fn restore_refuses_a_newer_schema() {
    let dir = tempfile::tempdir().unwrap();
    let archive_path = write_min_archive(dir.path(), BACKUP_FORMAT_VERSION, 2);
    let target_dir = tempfile::tempdir().unwrap();
    let factory = SqliteVaultRepositoryFactory::new();
    let err = restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: target_dir.path().join("s.vdb"),
            archive_path,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::UnsupportedSchemaVersion(2)),
        "schema 2 > 1 → refuse, got {err:?}"
    );
    assert_eq!(std::fs::read_dir(target_dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn restore_refuses_an_unknown_format() {
    let dir = tempfile::tempdir().unwrap();
    let archive_path = write_min_archive(dir.path(), 2, 1);
    let target_dir = tempfile::tempdir().unwrap();
    let factory = SqliteVaultRepositoryFactory::new();
    let err = restore_vault(
        &factory,
        &MemoryKeychainProvider::new(),
        RestoreVaultInput {
            target_vault: target_dir.path().join("f.vdb"),
            archive_path,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::MalformedPayload(_)),
        "format 2 → refuse, got {err:?}"
    );
    assert_eq!(std::fs::read_dir(target_dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn inspect_backup_reports_counts_and_flags() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("i.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    // Preview against a fresh (absent) target → unreadable, no hard-stops.
    let target_dir = tempfile::tempdir().unwrap();
    let preview = inspect_backup(InspectBackupInput {
        target_vault: target_dir.path().join("none.vdb"),
        archive_path: dest,
    })
    .await
    .unwrap();
    assert_eq!(preview.backup_entry_count, 2);
    assert_eq!(preview.format_version, BACKUP_FORMAT_VERSION);
    assert!(!preview.unknown_format);
    assert!(!preview.unknown_schema);
    assert!(!preview.uuid_mismatch);
    assert!(preview.target_unreadable);
    assert!(preview.rollback_delta.is_none());
}

/// T4 (5.2c): previewing an OLD backup against a live target that has moved on
/// reports the rollback delta the restore would cost.
#[tokio::test]
async fn inspect_reports_rollback_delta_against_a_live_ahead_target() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;

    // Snapshot the vault (its commit_counter goes into the manifest), then advance it.
    let backup_counter = h.repo.load_config().await.unwrap().commit_counter;
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("r.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    add(&mut session, "c").await;
    add(&mut session, "d").await;
    add(&mut session, "e").await;
    let target_counter = h.repo.load_config().await.unwrap().commit_counter;
    assert!(target_counter > backup_counter, "the live vault advanced");

    // Preview against the live (WAL) target — a mode=ro read coexists with the session.
    let preview = inspect_backup(InspectBackupInput {
        target_vault: h.vdb_path.clone(),
        archive_path: dest,
    })
    .await
    .unwrap();

    assert_eq!(preview.target_commit_counter, Some(target_counter));
    assert_eq!(
        preview.rollback_delta,
        Some(target_counter - backup_counter),
        "restoring the older backup would drop (target − backup) commits"
    );
    assert!(!preview.uuid_mismatch, "same vault → no uuid mismatch");
    assert!(!preview.target_unreadable);
}

/// T3 (5.2c): restoring an OLDER backup over a newer target is refused without
/// `confirm_rollback`, allowed with it, and re-bases the keychain mirror to the
/// restored counter. The target's counter comes from a VACUUM snapshot (an install of
/// a higher backup), never a bare file copy — a copy would miss uncheckpointed WAL.
#[tokio::test]
async fn restore_confirm_gate_refuses_then_rebaselines() {
    const UUID: &str = "01CCCCCCCCCCCCCCCCCCCCCCCC";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let mut session = unlock(&h).await;

    // A low backup (counter after 2 writes) and a high backup (after 5).
    add(&mut session, "a").await;
    add(&mut session, "b").await;
    let low_counter = h.repo.load_config().await.unwrap().commit_counter;
    let out = tempfile::tempdir().unwrap();
    let low = out.path().join("low.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: low.clone(),
        },
    )
    .await
    .unwrap();

    add(&mut session, "c").await;
    add(&mut session, "d").await;
    add(&mut session, "e").await;
    let high_counter = h.repo.load_config().await.unwrap().commit_counter;
    assert!(high_counter > low_counter);
    let high = out.path().join("high.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: high.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let factory = SqliteVaultRepositoryFactory::new();
    let keychain = MemoryKeychainProvider::new();

    // Install the HIGH backup into a fresh target → the target is now at `high_counter`.
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("t.vdb");
    restore_vault(
        &factory,
        &keychain,
        RestoreVaultInput {
            target_vault: target.clone(),
            archive_path: high,
            confirm_rollback: false,
        },
    )
    .await
    .unwrap();

    // (a) Restoring the LOW backup over it without confirmation → the rollback gate refuses.
    let err = restore_vault(
        &factory,
        &keychain,
        RestoreVaultInput {
            target_vault: target.clone(),
            archive_path: low.clone(),
            confirm_rollback: false,
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::MalformedPayload(_)),
        "rollback without confirm → refuse, got {err:?}"
    );

    // (b) With confirmation it restores and re-bases the keychain to the restored counter.
    restore_vault(
        &factory,
        &keychain,
        RestoreVaultInput {
            target_vault: target,
            archive_path: low,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        keychain.read_commit_baseline(UUID).unwrap(),
        Some(low_counter),
        "a confirmed restore re-bases the keychain to the restored (low) counter"
    );
}
