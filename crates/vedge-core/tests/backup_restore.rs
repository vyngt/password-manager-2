//! Backup + **Replace** host suite (slices 5.2a / 5.2b / 5.2.2).
//!
//! 5.2a proves the archive is produced live and well-formed. The rest proves the demoted
//! escape hatch — `replace_vault_from_backup` — refuses everything it should and, when it
//! does act, is **undoable**.
//!
//! 🔴 The non-destructive verb (`open_backup`) has its own suite in `tests/open_backup.rs`.
//! That separation is the slice: a test that "restores into a fresh path" is testing *Open
//! backup*, and it belongs over there. Replace **refuses** a missing target on purpose.
//!
//! The crash-at-every-state matrix is a unit test in `infrastructure::backup::journal` (it
//! drives the state machine directly, more precisely than a use-case fault point could).

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
use std::sync::Arc;

use zeroize::Zeroizing;

use common::{Harness, build_unlock};

use vedge_core::application::vault::ports::{
    BiometricAuthenticator, KeyDerivationProvider, KeychainProvider, VaultRepository,
};
use vedge_core::application::vault::session::VaultSession;
use vedge_core::application::vault::use_cases::{
    BackupVaultInput, ChangePasswordInput, CreateEntryInput, InspectBackupInput, OpenBackupInput,
    ReplaceVaultInput, RevertToSnapshotInput, TargetStateKind, UnlockVaultInput, backup_vault,
    change_password, create_entry, inspect_backup, open_backup, replace_vault_from_backup,
    revert_to_snapshot,
};
use vedge_core::domain::shared::{EntryId, SNAPSHOTS_DIR, VAULT_FILE};
use vedge_core::domain::vault::entities::{AuditAction, CURRENT_SCHEMA_VERSION};
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
            vault_path: h.home.clone(),
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

/// Stamp a specific `vault_uuid` onto a harness vault (the default harness uuid is a
/// random ULID, which can't exercise the mismatch path). Re-seeds the keychain under
/// the new uuid so a subsequent keychain-path unlock still resolves the Secret Key
/// (the keychain is uuid-keyed as of slice 5.2.0).
async fn set_uuid(h: &Harness, uuid: &str) {
    let mut cfg = h.config.clone();
    cfg.vault_uuid = Some(uuid.to_owned());
    h.repo.save_config(&cfg).await.unwrap();
    h.keychain.store_secret_key(uuid, &h.secret_key).unwrap();
}

/// Rotate the vault's master password (the ports are explicit at the use case; this keeps the
/// M3 tests readable).
async fn rotate_password(h: &Harness, session: &mut VaultSession, new: &str) {
    change_password(
        session,
        Arc::clone(&h.kdf) as Arc<dyn KeyDerivationProvider>,
        Arc::clone(&h.keychain) as Arc<dyn KeychainProvider>,
        Arc::clone(&h.biometric) as Arc<dyn BiometricAuthenticator>,
        ChangePasswordInput {
            new_password: Zeroizing::new(new.to_owned()),
            new_secret_key: None,
        },
    )
    .await
    .unwrap();
}

/// A Replace with nothing acknowledged — the default a UI submits when its preview showed no
/// risks. Every refusal test starts here and turns on exactly the one confirm it is probing,
/// which is the point of ③: **three independent risks, three independent acknowledgements.**
fn replace(target: &Path, archive: &Path) -> ReplaceVaultInput {
    ReplaceVaultInput {
        target_vault: target.to_path_buf(),
        archive_path: archive.to_path_buf(),
        confirm_rollback: false,
        confirm_credential_change: false,
        confirm_unverified_target: false,
    }
}

/// Materialise an archive at a fresh path (the *Open backup* verb), with no known vaults —
/// i.e. the new-machine case, where nothing can be a duplicate. Used to SET UP Replace tests
/// that need a populated target; the verb itself is proven in `tests/open_backup.rs`.
async fn open_at(archive: &Path, dest: &Path, keychain: &MemoryKeychainProvider) {
    open_backup(
        keychain,
        OpenBackupInput {
            archive_path: archive.to_path_buf(),
            dest_home: dest.to_path_buf(),
            known_vaults: vec![],
        },
    )
    .await
    .unwrap();
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
        verify_hash_prefix: None,
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

/// 🔴 **Replace REFUSES a missing target.** Not an error to confirm away — a different verb.
/// The whole of Decision ⑦ in one assertion: if there is nothing there, you wanted *Open
/// backup*, and the UI must send you there rather than quietly creating a vault behind a
/// dialog that said "replace".
#[tokio::test]
async fn replace_refuses_a_missing_target() {
    let h = Harness::fresh().await;
    let session = unlock(&h).await;
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("m.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("nothing-here.vedge");
    let factory = SqliteVaultRepositoryFactory::new();
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        replace(&target, &dest),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::TargetMissing),
        "an absent target is Open-backup's job, got {err:?}"
    );
    assert!(
        !target.exists(),
        "a refused replace must not create a vault"
    );
}

/// Replace OVER an existing (locked, matching-uuid) vault — exercises the move-aside + `.old`
/// cleanup and, on Windows, the close-the-read-handle-before-rename path.
///
/// ⚠️ **This test used to build its target by copying `vault.vdb` alone, out from under a live
/// session** — so the config rows were still sitting in the uncheckpointed `-wal` and the copy
/// was *unreadable*. It passed anyway, because 5.2b's unreadable-target fail-open skipped every
/// identity check (**that is finding H2**). The test believed it was proving "an existing target
/// with a matching uuid is replaced"; it was in fact proving that an unidentifiable target is
/// replaced without question. Now the target is built by OPENING the backup — a genuine,
/// readable, matching-uuid vault — so the guards it claims to pass are actually running.
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

    let keychain = MemoryKeychainProvider::new();
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("existing.vedge");
    open_at(&dest, &target, &keychain).await;

    let factory = SqliteVaultRepositoryFactory::new();
    replace_vault_from_backup(&factory, &keychain, replace(&target, &dest))
        .await
        .unwrap();

    // No transient artifacts survive.
    assert!(!target_dir.path().join("existing.vedge.old").exists());
    assert!(
        !target_dir
            .path()
            .join(".existing.vedge.restore-staging")
            .exists()
    );
    assert!(
        !target_dir
            .path()
            .join(".existing.vedge.restore.intent")
            .exists()
    );

    // The restored vault opens and carries the backed-up entry.
    let restored = unlock_at(&h, &target).await;
    drop(restored);
    let db = VaultDbConnection::open(&target.join(VAULT_FILE))
        .await
        .unwrap();
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
    let target = target_dir.path().join("t.vedge");
    let factory = SqliteVaultRepositoryFactory::new();
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        replace(&target, &dest),
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

    // 🔴 A HARD refusal — and note there is no `confirm_wrong_vault` to turn on. Every OTHER
    // risk on this path has an acknowledgement; this one does not, because there is no
    // legitimate reason to write vault B's contents over vault A.
    let factory = SqliteVaultRepositoryFactory::new();
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        ReplaceVaultInput {
            // Everything the user could possibly say yes to, said yes to. It still refuses.
            confirm_rollback: true,
            confirm_credential_change: true,
            confirm_unverified_target: true,
            ..replace(&hb.home, &dest)
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            VaultError::BackupWrongVault { ref backup, ref target }
                if backup == "01AAAAAAAAAAAAAAAAAAAAAAAA" && target == "01BBBBBBBBBBBBBBBBBBBBBBBB"
        ),
        "uuid mismatch → an unconfirmable BackupWrongVault, got {err:?}"
    );
}

#[tokio::test]
async fn restore_refuses_a_newer_schema() {
    let dir = tempfile::tempdir().unwrap();
    // A backup one schema version NEWER than this build understands — must be refused.
    let newer = CURRENT_SCHEMA_VERSION + 1;
    let archive_path = write_min_archive(dir.path(), BACKUP_FORMAT_VERSION, newer);
    let target_dir = tempfile::tempdir().unwrap();
    let factory = SqliteVaultRepositoryFactory::new();
    // The format/schema gates fire BEFORE the target is even looked at — so this refuses on
    // the schema, not on `TargetMissing`. That ordering matters: a user pointing a
    // from-the-future backup at anything must hear about the format, not the path.
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        replace(&target_dir.path().join("s.vedge"), &archive_path),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::UnsupportedSchemaVersion(v) if v == newer),
        "a schema newer than CURRENT → refuse, got {err:?}"
    );
    assert_eq!(std::fs::read_dir(target_dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn restore_refuses_a_newer_format() {
    let dir = tempfile::tempdir().unwrap();
    let archive_path = write_min_archive(dir.path(), BACKUP_FORMAT_VERSION + 1, 1);
    let target_dir = tempfile::tempdir().unwrap();
    let factory = SqliteVaultRepositoryFactory::new();
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        replace(&target_dir.path().join("f.vedge"), &archive_path),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            VaultError::BackupUnsupportedFormat(v) if v == BACKUP_FORMAT_VERSION + 1
        ),
        "a NEWER format → refuse, got {err:?}"
    );
    assert_eq!(std::fs::read_dir(target_dir.path()).unwrap().count(), 0);
}

/// Repack an existing `.vbk` with a doctored manifest — the members are re-hashed, so
/// the result is internally consistent and passes `verify_archive`. Only the manifest's
/// own metadata changes.
fn repack_with(
    archive_path: &Path,
    work: &Path,
    mutate: impl FnOnce(&mut BackupManifest),
) -> std::path::PathBuf {
    let mut manifest = archive::read_manifest(archive_path).unwrap();
    let mut members = Vec::new();
    for file in &manifest.files {
        let dest = work.join(file.name.replace('/', "_"));
        archive::extract_member(archive_path, &file.name, &dest).unwrap();
        members.push((file.name.clone(), dest));
    }
    mutate(&mut manifest);
    let manifest_path = work.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let mut all = vec![("manifest.json".to_owned(), manifest_path)];
    all.extend(members);
    let dest = work.join("repacked.vbk");
    archive::write_archive(&all, &dest).unwrap();
    dest
}

/// 🔴 **Finding H1 — the pin.** The guard is `>`, not `!=`: a backup written by an OLDER
/// build must still open, **forever**. This is the half nobody writes, and it is the half that
/// matters — the day `BACKUP_FORMAT_VERSION` becomes 2, a `!=` guard would refuse every `.vbk`
/// on every user's disk as "unknown format", inverting the entire purpose of a self-describing
/// manifest and breaking the folder's standing rule: *once a backup exists on a user's disk,
/// every future version must read it.*
///
/// Proven end-to-end on a REAL vault (not a dummy archive), so it also proves the older archive
/// genuinely opens afterwards — a refusal that merely moved downstream would not pass.
#[tokio::test]
async fn an_older_format_still_opens_forever() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("old.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // Forge a manifest claiming format_version 0 — "written by a build older than any that
    // ever shipped". It must still open.
    let work = tempfile::tempdir().unwrap();
    let old = repack_with(&dest, work.path(), |m| m.format_version = 0);

    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("old.vedge");
    let report = open_backup(
        &MemoryKeychainProvider::new(),
        OpenBackupInput {
            archive_path: old,
            dest_home: target.clone(),
            known_vaults: vec![],
        },
    )
    .await
    .expect("an OLDER format_version must open — H1");
    assert_eq!(report.entry_count, 1);
    assert!(target.join(VAULT_FILE).is_file());
}

/// Take a crash-image of a live vault home: `vault.vdb` + its **uncheckpointed `-wal`**,
/// with no `-shm` and no open handle — exactly what a killed process leaves behind.
///
/// Called while the harness still holds its connection open, so `SQLite` has NOT had the
/// chance to checkpoint and delete the WAL. (`-shm` is deliberately not copied; it is
/// rebuilt on the next open.)
fn crash_image_of(home: &Path, dest: &Path) {
    std::fs::create_dir_all(dest.join("blobs")).unwrap();
    std::fs::create_dir_all(dest.join(SNAPSHOTS_DIR)).unwrap();
    std::fs::copy(home.join(VAULT_FILE), dest.join(VAULT_FILE)).unwrap();

    let wal_name = format!("{VAULT_FILE}-wal");
    let wal = home.join(&wal_name);
    let wal_len = std::fs::metadata(&wal).map_or(0, |m| m.len());
    // 🔴 Guard the premise. If the WAL were empty (or absent), this test would still
    // pass while proving nothing at all — the exact failure mode where a regression
    // test quietly stops testing.
    assert!(
        wal_len > 0,
        "premise broken: the live vault has no dirty -wal to copy, so this test would \
         be vacuous. (Did WAL mode stop being pinned in VaultDbConnection::open?)"
    );
    std::fs::copy(&wal, dest.join(&wal_name)).unwrap();
    assert!(
        !dest.join(format!("{VAULT_FILE}-shm")).exists(),
        "a crash image must not carry a -shm"
    );
}

/// 🔴 **Finding H2.** A vault whose process was killed mid-write leaves a dirty `-wal`. 5.2
/// collapsed *"no vault here"* and *"a vault I could not read"* into one `None`, and restore
/// then skipped **every** identity check on that `None` — so **the uuid-mismatch guard
/// silently disarmed itself on precisely the vaults most likely to be restored over.**
///
/// The property that must hold: a crashed vault stays **identified**, so the guard stays
/// **armed**. Asserted on the outcome, never on which connection mode delivered it — the fix
/// must survive `SQLite`'s version-specific read-only WAL behaviour either way.
///
/// (The 5.2.2 spec proposed a `mode=rw` fallback on the theory that `mode=ro` cannot replay a
/// dirty WAL. Measured — it can. See the `target` module docs.)
#[tokio::test]
async fn a_dirty_wal_target_stays_readable_and_identified() {
    use vedge_core::infrastructure::backup::target::{TargetState, read_target_state};

    let h = Harness::fresh().await;
    set_uuid(&h, "01VAULTAAAAAAAAAAAAAAAAAAA").await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;
    add(&mut session, "b").await;

    // Image the home while the connection is STILL OPEN → the WAL cannot have been
    // checkpointed away.
    let crash_dir = tempfile::tempdir().unwrap();
    let crashed = crash_dir.path().join("crashed.vedge");
    crash_image_of(&h.home, &crashed);

    let state = read_target_state(&crashed).await;
    let TargetState::Readable(id) = state else {
        panic!("a dirty-WAL vault must still be identified, got {state:?}");
    };
    assert_eq!(
        id.vault_uuid.as_deref(),
        Some("01VAULTAAAAAAAAAAAAAAAAAAA"),
        "the uuid guard's input must survive a dirty WAL — H2"
    );
    assert_eq!(id.entry_count, 2, "the WAL's frames were replayed");

    // 🔴 And now the half that H2 was actually ABOUT: the guard must still FIRE. Point a
    // DIFFERENT vault's backup at this crashed one — if the crash had made it "unreadable",
    // 5.2b would have skipped the uuid check entirely and cheerfully overwritten it.
    let hb = Harness::fresh().await;
    set_uuid(&hb, "01VAULTBBBBBBBBBBBBBBBBBBB").await;
    let sb = unlock(&hb).await;
    let out = tempfile::tempdir().unwrap();
    let b_backup = out.path().join("b.vbk");
    backup_vault(
        &sb,
        BackupVaultInput {
            dest_archive: b_backup.clone(),
        },
    )
    .await
    .unwrap();
    drop(sb);

    let factory = SqliteVaultRepositoryFactory::new();
    let err = replace_vault_from_backup(
        &factory,
        &MemoryKeychainProvider::new(),
        ReplaceVaultInput {
            // Even with the unverified-target escape hatch armed, the guard fires — because the
            // target is NOT unverified. It read fine. That is the fix.
            confirm_unverified_target: true,
            confirm_rollback: true,
            confirm_credential_change: true,
            ..replace(&crashed, &b_backup)
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::BackupWrongVault { .. }),
        "a crashed vault must keep its identity, and the guard must keep firing, got {err:?}"
    );
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

    // Preview against a fresh (absent) target → MISSING (not "unreadable"), no hard-stops.
    let target_dir = tempfile::tempdir().unwrap();
    let preview = inspect_backup(InspectBackupInput {
        archive_path: dest,
        target_vault: Some(target_dir.path().join("none.vedge")),
        dest_home: None,
        known_vaults: vec![],
    })
    .await
    .unwrap();
    assert_eq!(preview.backup_entry_count, 2);
    assert_eq!(preview.format_version, BACKUP_FORMAT_VERSION);
    assert!(!preview.unknown_format);
    assert!(!preview.unknown_schema);
    assert!(!preview.uuid_mismatch);
    // 🔴 H2: absent is `Missing`, NOT `Unreadable`. Collapsing the two is the bug.
    assert_eq!(preview.target_state, TargetStateKind::Missing);
    assert!(
        preview.target_entry_count.is_none(),
        "an unknown count is None — never 0. A zero the user believes is a zero they act on."
    );
    assert!(preview.rollback_delta.is_none());
}

/// 🔴 **M3.** Back up, rotate the master password, then preview: the backup must be flagged as
/// needing the OLD credentials — *before* the user spends ten minutes discovering it.
///
/// And the other half, which is the one that protects people: an archive that predates the
/// check reports **unknown**, never *"same"*. Asserting a match you never verified is how a
/// user shreds the only Emergency Kit that could still open their backups.
#[tokio::test]
async fn m3_flags_a_credential_change_and_never_guesses() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "a").await;

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("cred.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: dest.clone(),
        },
    )
    .await
    .unwrap();

    // Same credentials on both sides → a VERIFIED match. Only now may it say `Some(false)`.
    let same = inspect_backup(InspectBackupInput {
        archive_path: dest.clone(),
        target_vault: Some(h.home.clone()),
        dest_home: None,
        known_vaults: vec![],
    })
    .await
    .unwrap();
    assert_eq!(same.credentials_differ, Some(false));

    // Rotate the master password → the archive now needs the OLD one.
    rotate_password(&h, &mut session, "an-entirely-different-master-phrase").await;
    drop(session);

    let changed = inspect_backup(InspectBackupInput {
        archive_path: dest.clone(),
        target_vault: Some(h.home.clone()),
        dest_home: None,
        known_vaults: vec![],
    })
    .await
    .unwrap();
    assert_eq!(
        changed.credentials_differ,
        Some(true),
        "M3: the backup was sealed under the old password and must say so"
    );

    // 🔴 A pre-5.2.2 archive carries no prefix. UNKNOWN — never `Some(false)`.
    let work = tempfile::tempdir().unwrap();
    let legacy = repack_with(&dest, work.path(), |m| m.verify_hash_prefix = None);
    let unknown = inspect_backup(InspectBackupInput {
        archive_path: legacy,
        target_vault: Some(h.home.clone()),
        dest_home: None,
        known_vaults: vec![],
    })
    .await
    .unwrap();
    assert_eq!(
        unknown.credentials_differ, None,
        "no prefix ⇒ UNKNOWN. Never assert a match you did not verify."
    );
}

/// 🔴 **② is an OPEN-mode question only.** Regression guard.
///
/// On the Replace path the target *is* the same vault — that is the operation's whole
/// precondition. A duplicate scan there finds the target itself and reports it as a "duplicate",
/// and the UI then tells the user they are making a *separate copy with its own identity* while
/// they are in fact **overwriting the original**. Two verbs, two questions.
#[tokio::test]
async fn a_replace_preview_never_calls_its_own_target_a_duplicate() {
    const UUID: &str = "01SELFSELFSELFSELFSELFSELF";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let session = unlock(&h).await;
    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("self.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();

    // Replace mode: the target is in `known_vaults` (it comes from recents, which of course
    // lists the vault the user just selected) and carries the very uuid in the manifest.
    let replace_preview = inspect_backup(InspectBackupInput {
        archive_path: vbk.clone(),
        target_vault: Some(h.home.clone()),
        dest_home: None,
        known_vaults: vec![h.home.clone()],
    })
    .await
    .unwrap();
    assert!(
        replace_preview.duplicate_of.is_none(),
        "replacing a vault with its OWN backup is not making a copy of it"
    );
    assert!(!replace_preview.uuid_mismatch, "and it is the right vault");

    // Open mode, same inputs: now it IS a duplicate, and must say so.
    let open_preview = inspect_backup(InspectBackupInput {
        archive_path: vbk,
        target_vault: None,
        dest_home: Some(out.path().join("copy.vedge")),
        known_vaults: vec![h.home.clone()],
    })
    .await
    .unwrap();
    assert_eq!(
        open_preview.duplicate_of.as_deref(),
        Some(h.home.as_path()),
        "opening a backup of a live vault at a new path IS making a copy"
    );
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
        archive_path: dest,
        target_vault: Some(h.home.clone()),
        dest_home: None,
        known_vaults: vec![],
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
    assert_eq!(preview.target_state, TargetStateKind::Readable);
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

    // Set up: OPEN the high backup at a fresh path (the non-destructive verb) → the target is
    // now at `high_counter`, and it is a real, readable, matching-uuid vault.
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("t.vedge");
    open_at(&high, &target, &keychain).await;

    // (a) Replacing with the LOW backup, unconfirmed → the rollback gate refuses.
    let err = replace_vault_from_backup(&factory, &keychain, replace(&target, &low))
        .await
        .unwrap_err();
    assert!(
        matches!(err, VaultError::RollbackNotConfirmed),
        "rollback without confirm → a dedicated refusal, got {err:?}"
    );

    // (b) With confirmation it replaces and re-bases the keychain to the restored counter.
    let report = replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            confirm_rollback: true,
            ..replace(&target, &low)
        },
    )
    .await
    .unwrap();
    assert_eq!(
        keychain.read_commit_baseline(UUID).unwrap(),
        Some(low_counter),
        "a confirmed replace re-bases the keychain to the restored (low) counter"
    );
    // ⑭: and it left an undo point behind.
    assert!(
        report.undo_snapshot_id.is_some(),
        "even the escape hatch must be undoable"
    );
}

// ---- 5.2.2: Replace is undoable, and the undo survives the swap ------------------

/// Give the vault at `target` a snapshot store holding `marker`, so a subsequent Replace can be
/// checked for having eaten it.
fn seed_snapshot_store(target: &Path, marker: &str) {
    let store = target.join(SNAPSHOTS_DIR);
    std::fs::create_dir_all(store.join("objects")).unwrap();
    std::fs::write(store.join(marker), b"a snapshot the user is relying on").unwrap();
}

/// 🔴🔴 **The bug this slice exists to have found.**
///
/// `restore_vault` committed through `journal::commit`, which swaps the WHOLE home. Since
/// 5.2.1 the snapshot store lives at `<home>/snapshots` — **inside** that home. So every
/// `.vbk` restore silently **deleted every snapshot the vault had**, including the
/// `pre-restore` undo point taken seconds earlier to make the operation reversible. ⑭'s
/// promise ("even the escape hatch is undoable") was, in shipped code, a lie.
///
/// Nothing caught it because no test asserted the continued existence of a directory nobody
/// was thinking about. This one does, and it is why `journal::commit` no longer exists.
#[tokio::test]
async fn replace_preserves_the_snapshot_store() {
    const UUID: &str = "01PRESERVEPRESERVEPRESERVE";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;

    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("p.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let keychain = MemoryKeychainProvider::new();
    let factory = SqliteVaultRepositoryFactory::new();
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("live.vedge");
    open_at(&vbk, &target, &keychain).await;

    // The user has snapshots. They are the safety net. They are inside the home.
    seed_snapshot_store(&target, "20260101T000000000Z");

    let report = replace_vault_from_backup(&factory, &keychain, replace(&target, &vbk))
        .await
        .unwrap();

    // 🔴 The store survived the whole-home swap.
    let store = target.join(SNAPSHOTS_DIR);
    assert!(
        store.join("20260101T000000000Z").is_file(),
        "the Replace swap DELETED the user's snapshot store — commit_preserving(&[SNAPSHOTS_DIR]) \
         is not being used"
    );
    assert!(
        store.join("objects").is_dir(),
        "the object pool survived too"
    );

    // And ⑭'s undo point — written into that same store moments before the swap — is still
    // there. If the store were wiped, this would be gone with it, and the "undoable" claim
    // would be false in exactly the case where it matters.
    let undo = report.undo_snapshot_id.expect("⑭ an undo point was taken");
    assert!(
        store.join(&undo).is_dir(),
        "the pre-restore undo point did not survive its own swap"
    );
}

/// **#9 / ⑭.** The undo is not just present — it *works*. Replace over a vault that has moved
/// on, then revert to the auto-snapshot and get the pre-replace state back, exactly.
#[tokio::test]
async fn the_replace_undo_point_restores_the_exact_pre_replace_state() {
    const UUID: &str = "01UNDOUNDOUNDOUNDOUNDOUNDO";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let mut session = unlock(&h).await;
    add(&mut session, "in-the-backup").await;

    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("u.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let keychain = MemoryKeychainProvider::new();
    let factory = SqliteVaultRepositoryFactory::new();
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("live.vedge");
    open_at(&vbk, &target, &keychain).await;

    // The live vault moves on: it now holds work that the backup does not.
    {
        let mut live = unlock_at(&h, &target).await;
        add(&mut live, "precious-unbacked-up-work").await;
        drop(live);
    }
    let entries_before = count_entries(&target).await;
    assert_eq!(entries_before, 2);

    // Replace it with the (older, 1-entry) backup. The user confirms the rollback.
    let report = replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            confirm_rollback: true,
            ..replace(&target, &vbk)
        },
    )
    .await
    .unwrap();
    assert_eq!(count_entries(&target).await, 1, "the replace took effect");

    // 🟢 Now undo it. This is the entire point of ⑭: the escape hatch did not have to be a
    // one-way door.
    let undo = report.undo_snapshot_id.expect("⑭ an undo point was taken");
    revert_to_snapshot(
        &factory,
        &keychain,
        RevertToSnapshotInput {
            vault: target.clone(),
            snapshot_id: undo,
            confirm_rollback: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        count_entries(&target).await,
        2,
        "the undo did not bring back the work the replace destroyed"
    );
}

/// Count active entries in a locked vault on disk, opening and closing a connection of its own
/// (so nothing is left holding the `.vdb` when the next swap renames it).
async fn count_entries(home: &Path) -> usize {
    let db = VaultDbConnection::open(&home.join(VAULT_FILE))
        .await
        .unwrap();
    let repo = SqliteVaultRepository::new(db.handle());
    let n = repo
        .all_entries()
        .await
        .unwrap()
        .iter()
        .filter(|e| !e.is_trashed)
        .count();
    drop(repo);
    db.close().await.unwrap();
    n
}

/// 🔴 **H2's acknowledgement.** An unreadable target cannot be identified, so the uuid guard
/// *could not run*. In 5.2b that silently allowed the replace. Now it is its own risk with its
/// own yes — and the vault is untouched until that yes arrives.
#[tokio::test]
async fn replace_refuses_an_unverified_target_until_it_is_confirmed() {
    let h = Harness::fresh().await;
    let mut session = unlock(&h).await;
    add(&mut session, "keeper").await;
    let out = tempfile::tempdir().unwrap();
    let vbk = out.path().join("uv.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: vbk.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    // A target that EXISTS but cannot be identified.
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("corrupt.vedge");
    std::fs::create_dir_all(target.join("blobs")).unwrap();
    std::fs::create_dir_all(target.join(SNAPSHOTS_DIR)).unwrap();
    let garbage = b"this is not a database".to_vec();
    std::fs::write(target.join(VAULT_FILE), &garbage).unwrap();

    let factory = SqliteVaultRepositoryFactory::new();
    let keychain = MemoryKeychainProvider::new();

    // Unconfirmed → refuse, and the corrupt target is byte-identical afterwards.
    let err = replace_vault_from_backup(&factory, &keychain, replace(&target, &vbk))
        .await
        .unwrap_err();
    assert!(
        matches!(err, VaultError::TargetUnverified),
        "an unidentifiable target must ASK, not assume, got {err:?}"
    );
    assert_eq!(
        std::fs::read(target.join(VAULT_FILE)).unwrap(),
        garbage,
        "a refused replace must leave the target byte-identical"
    );

    // 🔴 And the confirm is its OWN. Saying yes to the other two risks must NOT let this
    // through — one checkbox must never stand in for another.
    let err = replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            confirm_rollback: true,
            confirm_credential_change: true,
            ..replace(&target, &vbk)
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::TargetUnverified),
        "confirming OTHER risks must not confirm this one, got {err:?}"
    );

    // Confirmed → it proceeds, and the vault is real again.
    let report = replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            confirm_unverified_target: true,
            ..replace(&target, &vbk)
        },
    )
    .await
    .unwrap();
    assert_eq!(count_entries(&target).await, 1);
    assert!(
        report.undo_snapshot_id.is_none(),
        "there was nothing readable to snapshot — an undo point would be a fiction"
    );
}

/// 🔴 **M3's acknowledgement.** The backup was sealed under credentials the live vault no
/// longer uses, so replacing leaves a vault that will not open with today's password. Its own
/// risk, its own yes.
#[tokio::test]
async fn replace_refuses_a_credential_change_until_it_is_confirmed() {
    const UUID: &str = "01CREDCREDCREDCREDCREDCRED";
    let h = Harness::fresh().await;
    set_uuid(&h, UUID).await;
    let mut session = unlock(&h).await;
    add(&mut session, "old-creds").await;

    // (1) A backup under the OLD credentials.
    let out = tempfile::tempdir().unwrap();
    let old_creds = out.path().join("old-creds.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: old_creds.clone(),
        },
    )
    .await
    .unwrap();

    // (2) Rotate the master password, and back up again under the NEW ones.
    rotate_password(&h, &mut session, "a-brand-new-master-phrase").await;
    add(&mut session, "post-rotation").await;
    let new_creds = out.path().join("new-creds.vbk");
    backup_vault(
        &session,
        BackupVaultInput {
            dest_archive: new_creds.clone(),
        },
    )
    .await
    .unwrap();
    drop(session);

    let factory = SqliteVaultRepositoryFactory::new();
    let keychain = MemoryKeychainProvider::new();

    // (3) A live vault holding the NEW credentials. (Built at its own path — the harness still
    // holds a connection to `h.home`, and a swap of a vault with a live handle is exactly what
    // the product prevents by requiring a LOCKED target.)
    let target_dir = tempfile::tempdir().unwrap();
    let target = target_dir.path().join("live.vedge");
    open_at(&new_creds, &target, &keychain).await;

    // (4) Now try to put the OLD-credentials backup over it. After this swap the vault would
    // only open with a password the user has replaced — possibly one they no longer have.
    let err = replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            // The rollback risk IS acknowledged. The credential risk is NOT. It must still
            // refuse: one checkbox never stands in for another.
            confirm_rollback: true,
            ..replace(&target, &old_creds)
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, VaultError::BackupCredentialsDiffer),
        "a backup needing the OLD password must say so BEFORE it lands, got {err:?}"
    );

    replace_vault_from_backup(
        &factory,
        &keychain,
        ReplaceVaultInput {
            confirm_rollback: true,
            confirm_credential_change: true,
            ..replace(&target, &old_creds)
        },
    )
    .await
    .expect("with both risks acknowledged it proceeds");
    assert_eq!(
        count_entries(&target).await,
        1,
        "the old-creds backup landed"
    );
}
